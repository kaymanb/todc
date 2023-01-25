//! Checking [linearizability](https://en.wikipedia.org/wiki/Linearizability).  
//!
//! See _Testing for Linearizability_ by Gavin Lowe [[L16]](https://doi.org/10.1002/cpe.3928) and
//! _Faster Linearizability Checking via P-Compositionality_ by Horn and Kroening
//! [[HK15]](https://arxiv.org/abs/1504.00204).
//!
//! For a Go implementation, see [Porcupine](https://github.com/anishathalye/porcupine).
use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::Hash;
use std::rc::Rc;

use crate::linearizability::entry::Entry;

mod entry;

pub trait Specification {
    type State: Clone + Eq + Hash;
    type Operation;

    /// Returns an initial state for the specification.
    fn init() -> Self::State;

    /// Returns the result of applying the operation.
    fn apply(&self, operation: &Self::Operation, state: &Self::State) -> (bool, Self::State);
}

pub struct WLGChecker<S: Specification> {
    spec: S,
}

impl<S: Specification> WLGChecker<S> {
    /// Returns whether or not the input history is linearizable with respect
    /// to this checkers specification.
    pub fn is_linearizable(&self, mut entry: Rc<RefCell<Entry<S::Operation>>>) -> bool {
        true
        // let mut head = Entry::new(0);
        // head.next = Some(entry.clone());

        // let mut state = S::init();
        // let mut linearized = vec![false; entry.borrow().len() / 2];
        // let mut calls: Vec<(Entry<S::Operation>, S::State)> = Vec::new();
        // let mut cache: HashSet<(Vec<bool>, S::State)> = HashSet::new();
        // loop {
        //     if entry.borrow().is_last() {
        //         return true;
        //     }

        //     if entry.borrow().is_call() {
        //         let (is_linearizable, new_state) = self.spec.apply(&entry.borrow().operation.unwrap(), &state);
        //         let cache_copy = cache.clone();
        //         if is_linearizable {
        //             let mut lin_copy = linearized.clone();
        //             lin_copy[entry.borrow().id] = true;
        //             cache.insert((lin_copy, new_state));
        //         }
        //         if cache != cache_copy {
        //             // TODO: How to hash these...
        //             // calls.push((entry.borrow(), state));
        //             state = new_state;
        //             linearized[entry.borrow().id] = true;
        //             entry.borrow().lift();
        //             entry = head.next.unwrap();
        //         }
        //     }
        // }
    }
}
