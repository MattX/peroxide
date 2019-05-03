// Copyright 2018-2019 Matthieu Felix
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

//! Garbage collector.
//!
//! Simple and slow. Values are put into a large vector. A list of free cells is maintained.
//!
//! Values stored must implement the `Inventory` trait, which asks them to return a `Vec` of other
//! values they hold pointers to. This is not great, because we end up creating tons of tiny
//! vectors on the stack. Maybe it would be better to return a 5-tuple, as no values hold more than
//! that.
//!
//! The UnsafeCell business is used because we want to be able to add values to the GC while
//! references are being held. (You can insert in a non-mutable GC). Collection cannot happen
//! while values are being held, although really it shouldn't be a big issue either.

use std::cell::{RefCell, UnsafeCell};

pub struct PushOnlyVec<T> {
    underlying: Vec<T>,
}

impl<T> PushOnlyVec<T> {
    pub fn push(&mut self, v: T) {
        self.underlying.push(v);
    }

    fn get_vec(&mut self) -> &mut Vec<T> {
        &mut self.underlying
    }
}

pub trait Inventory {
    fn inventory(&self, v: &mut PushOnlyVec<usize>);
}

pub struct Gc<T: Inventory> {
    arena: UnsafeCell<Vec<Option<Box<T>>>>,
    free_cells: RefCell<Vec<usize>>,
}

impl<T: Inventory> Gc<T> {
    pub fn insert(&self, val: T) -> usize {
        let boxed = Some(Box::new(val));
        if let Some(insert_pos) = self.free_cells.borrow_mut().pop() {
            unsafe {
                (*self.arena.get())[insert_pos] = boxed;
            }
            insert_pos
        } else {
            unsafe {
                (*self.arena.get()).push(boxed);
                (*self.arena.get()).len() - 1
            }
        }
    }

    pub fn maybe_get(&self, pos: usize) -> Option<&T> {
        unsafe {
            if let Some(Some(ref r)) = (*self.arena.get()).get(pos) {
                Some(r)
            } else {
                None
            }
        }
    }

    pub fn get(&self, pos: usize) -> &T {
        self.maybe_get(pos).expect("get() on invalid GC value")
    }

    fn remove(&mut self, pos: usize) {
        if unsafe { std::mem::replace(&mut (*self.arena.get())[pos], None) }.is_some() {
            self.free_cells.borrow_mut().push(pos);
        }
    }

    pub fn collect(&mut self, roots: &[usize]) {
        let current_len = unsafe { (*self.arena.get()).len() };

        let mut marks = vec![false; current_len];
        let mut to_mark = PushOnlyVec {
            underlying: Vec::new(),
        };
        to_mark.get_vec().extend_from_slice(roots);

        while let Some(i) = to_mark.get_vec().pop() {
            if marks[i] {
                continue;
            }
            marks[i] = true;
            self.get(i).inventory(&mut to_mark);
        }

        for (i_m, mark) in marks.iter().enumerate() {
            if !mark {
                self.remove(i_m);
            }
        }
    }
}

impl<T: Inventory> Default for Gc<T> {
    fn default() -> Self {
        Gc {
            arena: UnsafeCell::new(Vec::new()),
            free_cells: RefCell::new(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Node {
        pub id: String,
        pub refs: Vec<usize>,
    }

    impl Node {
        fn new(id: String, refs: Vec<usize>) -> Self {
            Node { id, refs }
        }
    }

    impl Inventory for Node {
        fn inventory(&self, pv: &mut PushOnlyVec<usize>) {
            for v in self.refs.iter() {
                pv.push(*v);
            }
        }
    }

    impl Default for Node {
        fn default() -> Self {
            Node {
                id: "default".into(),
                refs: Vec::new(),
            }
        }
    }

    #[test]
    fn alloc() {
        let gc: Gc<Node> = Default::default();
        assert_eq!(gc.insert(Default::default()), 0);
        assert_eq!(gc.insert(Default::default()), 1);
    }

    #[test]
    fn alloc_get() {
        let gc: Gc<Node> = Default::default();
        assert_eq!(gc.insert(Default::default()), 0);
        assert_eq!(gc.insert(Node::new("Test".into(), vec![])), 1);
        assert_eq!(gc.insert(Default::default()), 2);
        assert_eq!(gc.get(1).id, "Test");
    }

    #[test]
    fn alloc_ref() {
        let gc: Gc<Node> = Default::default();
        assert_eq!(gc.insert(Node::new("0".into(), vec![])), 0);
        let first = gc.get(0);
        assert_eq!(gc.insert(Node::new("1".into(), vec![])), 1);
        let snd = gc.get(1);

        assert_eq!(first.id, "0");
        assert_eq!(snd.id, "1");
    }

    #[test]
    fn collect_all() {
        let mut gc: Gc<Node> = Default::default();
        assert_eq!(gc.insert(Default::default()), 0);
        assert_eq!(gc.insert(Default::default()), 1);
        gc.collect(&vec![]);
        assert_eq!(gc.insert(Default::default()), 1);
        assert_eq!(gc.insert(Default::default()), 0);
    }

    #[test]
    fn collect_not_roots() {
        let mut gc: Gc<Node> = Default::default();
        assert_eq!(gc.insert(Node::new("Root".into(), vec![])), 0);
        assert_eq!(gc.insert(Default::default()), 1);
        gc.collect(&vec![0]);
        assert_eq!(gc.get(0).id, "Root");
        assert_eq!(gc.insert(Default::default()), 1);
    }

    #[test]
    fn collect_not_graph() {
        // 0-> 1 -> 2 -> 0 is a rooted loop on 1
        // 3 <-> 4 is a non-rooted loop

        let mut gc: Gc<Node> = Default::default();
        assert_eq!(gc.insert(Node::new("0".into(), vec![1])), 0);
        assert_eq!(gc.insert(Node::new("1".into(), vec![2])), 1);
        assert_eq!(gc.insert(Node::new("2".into(), vec![0])), 2);
        assert_eq!(gc.insert(Node::new("3".into(), vec![4])), 3);
        assert_eq!(gc.insert(Node::new("4".into(), vec![3])), 4);
        gc.collect(&vec![0]);
        assert_eq!(gc.get(0).id, "0");
        assert_eq!(gc.get(1).id, "1");
        assert_eq!(gc.get(2).id, "2");
        assert!(gc.maybe_get(3).is_none());
        assert!(gc.maybe_get(4).is_none());
    }

    #[test]
    fn no_readress() {
        let gc: Gc<Node> = Default::default();
        gc.insert(Node::new("Label".into(), vec![]));
        let val = gc.get(0);
        for _i in 0..10_000 {
            gc.insert(Node::default());
        }
        assert_eq!(val.id, "Label");
    }
}
