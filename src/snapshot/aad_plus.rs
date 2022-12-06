//! Implementations of atomic snapshot objects based on the paper by
//! Afek, Attiya, Dolev, Gafni, Merritt and Shavit [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741). 
use super::Snapshot;
use crate::register::{Register, AtomicRegister};

#[derive(Clone, Copy)]
struct UnboundedContents<T: Copy, const N: usize> {
    data: T,
    sequence: usize,
    view: [T; N] // TODO: How to deal with no knowing this at compile time...
}

/// An atomic snapshot from unbounded single-writer multi-reader
/// atomic regisers. 
///
/// TODO: Explain why unbounded
pub struct UnboundedAtomicSnapshot<T: Copy, const N: usize> { 
    registers: Vec<AtomicRegister<UnboundedContents<T, N>>>,
}

impl<T: Copy, const N: usize> UnboundedAtomicSnapshot<T, N> {
    fn collect(&self) -> [T; N] {

    }
}

impl<T: Copy, const N: usize> Snapshot<N> for UnboundedAtomicSnapshot<T, N> {
    type Value = T;

    fn new(value: Self::Value) -> Self {
        let initial_contents = UnboundedContents {
            data: value,
            sequence: 0,
            view: [value; N]
        };
        Self {
            registers: vec![AtomicRegister::new(initial_contents); N]
        }
    }

    fn scan(&self) -> [Self::Value; N] {
        // TODO
    }

    fn update(&self, i: usize, value: Self::Value) -> () {
        // TODO
    }
}

/// An atomic snapshot from single-writer multi-reader
/// atomic registers.
pub struct BoundedAtomicSnapshot {

}
