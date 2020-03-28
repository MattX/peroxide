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

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Deref;

use num_bigint::BigInt;

use compile::CodeBlock;
use environment::{ActivationFrame, RcEnv};
use heap;
use heap::{PoolPtr, RootPtr};
use primitives::{Port, SyntacticClosure};
use value::Value;
use vm::Vm;

/// A frontend for Heap / RHeap that handles convenience operations.
///
/// Should be renamed MemoryManager or something.
pub struct Arena {
    /// Roots held by the arena. This must come before [`heap`], or the `Drop` on `RootPtr`
    /// will panic.
    /// Clippy thinks this is never used, but just holding it is what's important
    #[allow(dead_code)]
    roots: Vec<RootPtr>,
    symbol_map: RefCell<HashMap<String, RootPtr>>,
    gensym_counter: Cell<usize>,
    pub undefined: PoolPtr,
    pub unspecific: PoolPtr,
    pub eof: PoolPtr,
    pub empty_list: PoolPtr,
    pub t: PoolPtr,
    pub f: PoolPtr,
    heap: heap::RHeap,
}

impl Arena {
    /// Moves a value into the arena, and returns a pointer to its new position.
    pub fn insert(&self, v: Value) -> PoolPtr {
        match v {
            Value::Undefined => self.undefined,
            Value::Unspecific => self.unspecific,
            Value::EofObject => self.eof,
            Value::EmptyList => self.empty_list,
            Value::Boolean(true) => self.t,
            Value::Boolean(false) => self.f,
            Value::Symbol(s) => {
                let res = self.symbol_map.borrow().get(&s).cloned();
                match res {
                    Some(u) => u.pp(),
                    None => {
                        let label = s.clone();
                        let pos = self.heap.allocate_rooted(Value::Symbol(s));
                        let ptr = pos.pp();
                        self.symbol_map.borrow_mut().insert(label, pos);
                        ptr
                    }
                }
            }
            _ => self.heap.allocate(v),
        }
    }

    pub fn root(&self, at: PoolPtr) -> RootPtr {
        self.heap.root(at)
    }

    pub fn root_vm(&self, vm: &Vm) {
        self.heap.root_vm(vm);
    }

    pub fn unroot_vm(&self) {
        self.heap.unroot_vm();
    }

    pub fn insert_rooted(&self, v: Value) -> RootPtr {
        self.root(self.insert(v))
    }

    /// Given a PoolPtr, returns a reference to the value at that location.
    ///
    /// The magic is to make rust believe that the reference lasts for any possible lifetime.
    /// In most places in the code, this can be replaced with just &*at.
    pub fn get<'a>(&'a self, at: PoolPtr) -> &'a Value {
        unsafe { std::mem::transmute::<&Value, &'a Value>(&*at) }
    }

    pub fn get_activation_frame(&self, at: PoolPtr) -> &RefCell<ActivationFrame> {
        if let Value::ActivationFrame(ref af) = self.get(at) {
            af
        } else {
            panic!("value is not an activation frame");
        }
    }

    pub fn get_code_block(&self, at: PoolPtr) -> &CodeBlock {
        if let Value::CodeBlock(c) = self.get(at) {
            c
        } else {
            panic!("value is not a code block");
        }
    }

    pub fn gensym(&self, base: Option<&str>) -> PoolPtr {
        let base_str = base.map(|s| format!("{}-", s)).unwrap_or_else(|| "".into());
        loop {
            let candidate = format!("--gs-{}{}", base_str, self.gensym_counter.get());
            self.gensym_counter.set(self.gensym_counter.get() + 1);
            if !self.symbol_map.borrow().contains_key(&candidate) {
                return self.insert(Value::Symbol(candidate));
            }
        }
    }
}

impl Default for Arena {
    fn default() -> Self {
        let mut roots = Vec::new();
        let values = heap::RHeap::with_gc_mode(heap::GcMode::Normal);

        macro_rules! root {
            ($i: ident, $x: expr) => {
                roots.push(values.allocate_rooted($x));
                let $i = roots.last().unwrap().ptr;
            };
        }

        root!(undefined, Value::Undefined);
        root!(unspecific, Value::Unspecific);
        root!(eof, Value::EofObject);
        root!(empty_list, Value::EmptyList);
        root!(f, Value::Boolean(false));
        root!(t, Value::Boolean(true));

        Arena {
            heap: values,
            symbol_map: RefCell::new(HashMap::new()),
            gensym_counter: Cell::new(0),
            undefined,
            unspecific,
            eof,
            empty_list,
            f,
            t,
            roots,
        }
    }
}

#[cfg(test)]
mod tests {
    use value::Value;

    use super::*;

    #[test]
    fn get_symbol() {
        let r = Value::Symbol("abc".into());
        let arena = Arena::default();
        let vr = arena.insert(r.clone());
        assert_eq!(arena.get(vr), &r);
    }

    #[test]
    fn get_number() {
        let arena = Arena::default();
        let vr = arena.insert(Value::Real(0.1));
        assert_eq!(arena.get(vr), &Value::Real(0.1));
    }
}
