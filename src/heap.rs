// Copyright 2018-2020 Matthieu Felix
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// General strategy
///
/// We maintain a set of pools, each of which contains some number (256 currently) of entries.
/// Each entry is either empty, or contains a value. Empty entries are part of a linked list.
///
/// A Heap object manages pools. When we need to perform an allocation, we take the first pool
/// we find with at least one free entry, and make that entry occupied. We edit the free entry
/// linked list accordingly.
///
/// Each pool also has a bitvec for the mark phase of GC.
///
/// RootPtrs are special pointers that implement Drop. When they are dropped, they automatically
/// unroot themselves from the heap. PoolPtrs are regular pointers that do not require roots to
/// exist.
///
/// This means that access through a PoolPtr might cause a segfault if the root has actually already
/// been dropped.
use std::cell::UnsafeCell;
use std::convert::{From, TryFrom};
use std::fmt::{self, Debug, Error, Formatter};
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::str::FromStr;

use bitvec::prelude::{BitBox, BitVec};
use value::Value;
use vm::Vm;

const POOL_ENTRIES: u16 = 1 << 8;
const FIRST_GC: usize = 1024 * 1024;
const GC_GROWTH: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcMode {
    Off,
    DebugHeavy,
    DebugNormal,
    Normal,
}

impl GcMode {
    fn is_debug(self) -> bool {
        self == GcMode::DebugHeavy || self == GcMode::DebugNormal
    }

    fn is_normal(self) -> bool {
        self == GcMode::DebugNormal || self == GcMode::Normal
    }
}

impl FromStr for GcMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(GcMode::Off),
            "normal" => Ok(GcMode::Normal),
            "debug" => Ok(GcMode::DebugNormal),
            "debug-heavy" => Ok(GcMode::DebugHeavy),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FreePoolEntry {
    prev: Option<u16>,
    next: Option<u16>,
}

struct UsedPoolEntry(Value);

impl Debug for UsedPoolEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Debug)]
enum PoolEntry {
    Free(FreePoolEntry),
    Used(UsedPoolEntry),
}

impl PoolEntry {
    fn is_free(&self) -> bool {
        match self {
            PoolEntry::Free(_) => true,
            PoolEntry::Used(_) => false,
        }
    }
}

impl Default for PoolEntry {
    fn default() -> Self {
        PoolEntry::Free(FreePoolEntry {
            prev: None,
            next: None,
        })
    }
}

struct Pool {
    data: [PoolEntry; POOL_ENTRIES as usize],
    free_block: Option<u16>,
    allocated: u16,
    marked: BitBox,
}

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut data_string = "[".to_string();
        data_string.push_str(
            &self.data[..]
                .iter()
                .map(|pe| format!("{:?}", pe))
                .collect::<Vec<_>>()
                .join(", "),
        );
        data_string.push(']');
        f.debug_struct("Pool")
            .field("data", &data_string)
            .field("free", &self.free_block)
            .field("allocated", &self.allocated)
            .finish()
    }
}

impl Pool {
    fn new() -> Pin<Box<Self>> {
        let data = {
            let mut data: [MaybeUninit<PoolEntry>; POOL_ENTRIES as usize] =
                unsafe { MaybeUninit::uninit().assume_init() };

            for (i_block, item) in data.iter_mut().enumerate() {
                let i_block = u16::try_from(i_block).expect("wat");
                *item = MaybeUninit::new(PoolEntry::Free(FreePoolEntry {
                    prev: if i_block == 0 {
                        None
                    } else {
                        Some(i_block - 1)
                    },
                    next: if i_block == POOL_ENTRIES - 1 {
                        None
                    } else {
                        Some(i_block + 1)
                    },
                }));
            }

            unsafe { std::mem::transmute::<_, [PoolEntry; POOL_ENTRIES as usize]>(data) }
        };
        let pool = Pool {
            data,
            free_block: Some(0),
            allocated: 0,
            marked: BitVec::from(&[false; POOL_ENTRIES as usize][..]).into_boxed_bitslice(),
        };
        Box::pin(pool)
    }
}

impl Pool {
    fn allocate(self: Pin<&mut Self>, value: Value) -> Option<PoolPtr> {
        let selr = unsafe { self.get_unchecked_mut() };
        if let Some(old_free_index) = selr.free_block {
            // println!("allocating {:?} at {}", &value, old_free_index);
            let next = if let PoolEntry::Free(ref e) = selr.data[usize::from(old_free_index)] {
                e.next
            } else {
                panic!("free not pointing to free entry")
            };
            selr.data[usize::from(old_free_index)] = PoolEntry::Used(UsedPoolEntry(value));
            if let Some(next) = next {
                if let PoolEntry::Free(ref mut e) = selr.data[usize::from(next)] {
                    e.prev = None
                } else {
                    panic!("free->next not pointing to free entry")
                }
            }
            selr.free_block = next;
            selr.allocated += 1;
            Some(PoolPtr {
                pool: selr as *mut Pool,
                idx: old_free_index,
            })
        } else {
            debug_assert_eq!(selr.allocated, POOL_ENTRIES);
            None
        }
    }

    #[cfg(test)]
    fn free(self: Pin<&mut Self>, idx: u16, debug: bool) {
        let selr = unsafe { self.get_unchecked_mut() };
        selr.free_ref(idx, debug);
    }

    /// Frees the memory at the specified address by returning the memory to the free list.
    ///
    /// If the GC is in debug mode, the memory will be marked free, but not returned to the free
    /// list. This allows us to get a nice error, instead of a segmentation fault or garbage data,
    /// when freed memory is accessed again.
    fn free_ref(&mut self, idx: u16, debug: bool) {
        debug_assert!(
            !self.data[usize::from(idx)].is_free(),
            "freeing free entry!"
        );
        // println!("freeing {:?} at {:?} {}", self.data[usize::from(idx)], self as *const Self, idx);
        self.data[usize::from(idx)] = PoolEntry::Free(FreePoolEntry {
            prev: None,
            next: if debug { None } else { self.free_block },
        });

        if let Some(free_index) = self.free_block {
            if let PoolEntry::Free(ref mut f) = self.data[usize::from(free_index)] {
                debug_assert_eq!(f.prev, None);
                if !debug {
                    f.prev = Some(idx);
                }
            } else {
                panic!("free_block not pointing at free entry");
            }
        }
        if !debug {
            self.free_block = Some(idx);
            self.allocated -= 1;
        }
    }

    /// Returns the number of freed entries
    fn sweep(self: Pin<&mut Self>, debug: bool) -> u16 {
        let mut selr = unsafe { self.get_unchecked_mut() };
        let init = selr.allocated;
        for (i_mark, mark) in selr.marked.clone().iter().enumerate() {
            if !mark && !selr.data[i_mark].is_free() {
                /*
                if let PoolEntry::Used(UsedPoolEntry(Value::CodeBlock(_))) = &selr.data[i_mark] {
                    println!("Freeing code block at {:?} / {}", selr as *const Pool, i_mark);
                }
                */
                selr.free_ref(u16::try_from(i_mark).unwrap(), debug)
            }
        }
        selr.marked = BitVec::from(&[false; POOL_ENTRIES as usize][..]).into_boxed_bitslice();
        init - selr.allocated
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PoolPtr {
    pool: *mut Pool,
    idx: u16,
}

impl Copy for PoolPtr {}

impl Clone for PoolPtr {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool,
            idx: self.idx,
        }
    }
}

impl Deref for PoolPtr {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        let pool = unsafe { &*self.pool };
        match &pool.data[usize::from(self.idx)] {
            PoolEntry::Used(u) => &u.0,
            PoolEntry::Free(_) => panic!("dereferencing freed value at {:?}", self),
        }
    }
}

impl PoolPtr {
    /// Normally, you go from a PoolPtr to an &Value using deref() (or implicitly). However,
    /// in that case, Rust assumes that the &Value lives for as long as the PoolPtr. This is
    /// not true; unless there is a rooting issue, the &Value will likely live for much longer.
    ///
    /// This method just makes Rust understand the &Value lasts for as long as needed.
    // This triggers a Clippy lint that I'm pretty sure shouldn't trigger. Maybe a regression of
    // https://github.com/rust-lang/rust-clippy/issues/2719?
    #[allow(clippy::transmute_ptr_to_ptr)]
    pub fn long_lived<'a, 'b>(&'a self) -> &'b Value {
        unsafe { std::mem::transmute::<&Value, _>(&*self) }
    }
}

#[cfg(any(debug_assertions, test))]
impl PoolPtr {
    fn maybe_deref(&self) -> &PoolEntry {
        let pool = unsafe { &*self.pool };
        &pool.data[usize::from(self.idx)]
    }

    pub fn ok(&self) -> bool {
        self.idx < POOL_ENTRIES
    }
}

pub struct PtrVec(Vec<PoolPtr>);

impl PtrVec {
    pub fn push(&mut self, v: PoolPtr) {
        #[cfg(debug_assertions)]
        {
            debug_assert!(v.ok());
        }
        self.0.push(v);
    }

    fn get_vec(&mut self) -> &mut Vec<PoolPtr> {
        &mut self.0
    }
}

pub trait Inventory {
    fn inventory(&self, v: &mut PtrVec);
}

#[derive(Debug)]
struct Heap {
    pools: Vec<Pin<Box<Pool>>>,
    full_pools: Vec<Pin<Box<Pool>>>,
    roots: Vec<Option<PoolPtr>>,
    allocated_values: usize,
    next_gc: usize,
    gc_mode: GcMode,
    // `vms` basically acts as additional roots. There can be several rooted VMs at the same
    // time when `eval` is used.
    vms: Vec<*const Vm>,
}

impl Default for Heap {
    fn default() -> Self {
        Heap {
            pools: Vec::new(),
            full_pools: Vec::new(),
            roots: Vec::new(),
            allocated_values: 0,
            next_gc: FIRST_GC,
            gc_mode: GcMode::Off,
            vms: Vec::new(),
        }
    }
}

impl Heap {
    fn allocate(&mut self, v: Value) -> PoolPtr {
        if self.gc_mode == GcMode::DebugHeavy {
            self.gc();
        } else if self.gc_mode.is_normal() && self.allocated_values > self.next_gc {
            // println!("running GC");
            self.gc();
            // println!("ran GC");
            self.next_gc = (self.allocated_values as f32 * GC_GROWTH) as usize;
        }

        if self.pools.is_empty() {
            self.pools.push(Pool::new())
        }

        let last_pool = self.pools.last_mut().expect("no free pools");
        let ptr = last_pool
            .as_mut()
            .allocate(v)
            .expect("full pool in non-full list");
        let last_pool = &*last_pool;
        if last_pool.allocated == POOL_ENTRIES {
            let pool = self.pools.pop().unwrap();
            self.full_pools.push(pool);
        }
        self.allocated_values += 1;
        // println!("Allocated {:?} for {:?}", ptr, *ptr);
        ptr
    }

    fn root(&mut self, p: PoolPtr) -> usize {
        #[cfg(debug_assertions)]
        {
            debug_assert!(!p.maybe_deref().is_free(), "rooting freed pointer {:?}", p,);
        }
        let empty = self
            .roots
            .iter_mut()
            .enumerate()
            .find(|(_i, e)| e.is_none());
        match empty {
            Some((i_r, r)) => {
                // println!("rooted {:?} at {}", p, i_r);
                *r = Some(p);
                i_r
            }
            None => {
                // println!("rooted {:?} at {}", p, self.roots.len());
                self.roots.push(Some(p));
                self.roots.len() - 1
            }
        }
    }

    fn root_vm(&mut self, vm: &Vm) {
        self.vms.push(vm as *const Vm)
    }

    fn unroot_vm(&mut self) {
        self.vms.pop();
    }

    fn gc(&mut self) {
        let stack: Vec<_> = self.roots.iter().filter_map(|s| *s).collect();
        let mut stack = PtrVec(stack);

        for &v in self.vms.iter() {
            unsafe {
                (*v).inventory(&mut stack);
            }
        }

        while let Some(root) = stack.get_vec().pop() {
            let pool = unsafe { &mut *root.pool };
            if !pool.marked[usize::from(root.idx)] {
                // println!("Inventorying {}", *root);
                pool.marked.set(usize::from(root.idx), true);
                (*root).inventory(&mut stack);
            }
        }
        for pool in self.pools.iter_mut() {
            self.allocated_values -= usize::from(pool.as_mut().sweep(self.gc_mode.is_debug()));
        }
        for pool in self.full_pools.iter_mut() {
            self.allocated_values -= usize::from(pool.as_mut().sweep(self.gc_mode.is_debug()));
        }
        for i_pool in (0..self.full_pools.len()).rev() {
            if self.full_pools[i_pool].allocated != POOL_ENTRIES {
                let pool = self.full_pools.swap_remove(i_pool);
                self.pools.push(pool);
            }
        }
        self.pools.sort_by_key(|p| p.allocated)
    }
}

pub struct RHeap(Rc<UnsafeCell<Heap>>);

impl Default for RHeap {
    fn default() -> Self {
        RHeap(Rc::new(UnsafeCell::new(Heap::default())))
    }
}

impl RHeap {
    pub fn with_gc_mode(gc_mode: GcMode) -> RHeap {
        RHeap(Rc::new(UnsafeCell::new(Heap {
            pools: vec![],
            full_pools: vec![],
            roots: vec![],
            allocated_values: 0,
            next_gc: FIRST_GC,
            gc_mode,
            vms: vec![],
        })))
    }

    pub fn allocate(&self, v: Value) -> PoolPtr {
        unsafe { &mut *self.0.get() }.allocate(v)
    }

    pub fn root(&self, v: PoolPtr) -> RootPtr {
        let s = unsafe { &mut *self.0.get() };
        let idx = s.root(v);
        let heap = Rc::downgrade(&self.0);
        RootPtr { ptr: v, heap, idx }
    }

    pub fn root_vm(&self, vm: &Vm) {
        unsafe { &mut *self.0.get() }.root_vm(vm);
    }

    pub fn unroot_vm(&self) {
        unsafe { &mut *self.0.get() }.unroot_vm();
    }

    pub fn allocate_rooted(&self, v: Value) -> RootPtr {
        let ptr = self.allocate(v);
        self.root(ptr)
    }

    #[cfg(test)]
    fn gc(&self) {
        unsafe { &mut *self.0.get() }.gc()
    }
}

/// A rooted pointer. Will unroot itself when dropped.
#[derive(Debug)]
pub struct RootPtr {
    pub ptr: PoolPtr,
    heap: Weak<UnsafeCell<Heap>>,
    idx: usize,
}

impl Clone for RootPtr {
    fn clone(&self) -> Self {
        let rheap = RHeap(self.heap.upgrade().expect("heap destroyed"));
        rheap.root(self.ptr)
    }
}

impl Deref for RootPtr {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &*self.ptr
    }
}

impl Drop for RootPtr {
    fn drop(&mut self) {
        // TODO - another option is do just ignore dead heaps as there's no need to unroot.
        //        however, a destroyed heap can mean that we have other dangling pointers.
        unsafe { &mut *self.heap.upgrade().expect("heap destroyed").get() }.roots[self.idx] = None;
        // println!("unrooted {{ pool: {:p}, idx: {} }}", self.heap.upgrade().unwrap(), self.idx);
    }
}

impl RootPtr {
    pub fn pp(&self) -> PoolPtr {
        self.ptr
    }
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use crate::heap::{Pool, RHeap, POOL_ENTRIES};
    use crate::value::Value;

    #[test]
    fn test_alloc_free() {
        let reference = Value::String(RefCell::new("abcdef".to_string()));
        let mut pool = Pool::new();
        let ptr = pool
            .as_mut()
            .allocate(reference.clone())
            .expect("should have room");
        assert_eq!(pool.allocated, 1);
        assert_eq!(*ptr, reference);
        pool.as_mut().free(ptr.idx, false);
        assert_eq!(pool.allocated, 0);
    }

    #[test]
    fn test_alloc_dealloc_alloc() {
        let mut pool = Pool::new();
        pool.as_mut()
            .allocate(Value::Integer(0.into()))
            .expect("should have room");
        let ptr1 = pool
            .as_mut()
            .allocate(Value::Integer(1.into()))
            .expect("should have room");
        pool.as_mut()
            .allocate(Value::Integer(2.into()))
            .expect("should have room");
        assert_eq!(pool.allocated, 3);
        assert_eq!(*ptr1, Value::Integer(1.into()));
        pool.as_mut().free(ptr1.idx, false);
        assert_eq!(pool.allocated, 2);
        let ptr1b = pool
            .as_mut()
            .allocate(Value::Integer(3.into()))
            .expect("should have room");
        assert_eq!(ptr1b.idx, 1);
    }

    #[test]
    fn test_exhaust() {
        let val = Value::Integer(0.into());
        let mut pool = Pool::new();
        for _ in 0..POOL_ENTRIES {
            pool.as_mut()
                .allocate(val.clone())
                .expect("should have room");
        }
        assert_eq!(pool.as_mut().allocate(val.clone()), None);
        pool.as_mut().free(POOL_ENTRIES / 2, false);
        assert!(pool.as_mut().allocate(val).is_some());
    }

    #[test]
    fn test_alloc_heap() {
        let val = Value::Integer(0.into());
        let heap = RHeap::default();
        let val_ptr = heap.allocate(val.clone());
        assert_eq!(*val_ptr, val);
    }

    #[test]
    fn test_reclaim_unrooted() {
        let val = Value::Integer(0.into());
        let heap = RHeap::default();
        let val_ptr = heap.allocate(val.clone());
        assert_eq!(*val_ptr, val);
        heap.gc();
        assert!(val_ptr.maybe_deref().is_free());
    }

    #[test]
    fn test_dont_reclaim_rooted() {
        let val = Value::Integer(0.into());
        let heap = RHeap::default();
        let val_ptr = heap.allocate(val.clone());
        let rooted_ptr = heap.root(val_ptr);
        assert_eq!(*rooted_ptr, val);
        assert_eq!(*val_ptr, val);
        heap.gc();
        assert_eq!(*rooted_ptr, val);
        assert_eq!(*val_ptr, val);
        std::mem::drop(rooted_ptr);
        heap.gc();
        assert!(val_ptr.maybe_deref().is_free());
    }
}
