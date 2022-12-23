//! Implementations of atomic snapshot objects based on the paper by
//! Attiya and Rachman [[AR93]](https://doi.org/10.1137/S0097539795279463).
use super::Snapshot;

use crate::register::{AtomicRegister, Register};

#[derive(Clone, Copy)]
struct Contents<T: Copy, N: usize> {
    data: T,
    sequence: usize,
    counter: usize
}

// TODO: Document
pub struct AtomicSnapshot<T: Copy, N: usize, M: usize> {
    registers: [AtomicRegister<Contents<T, N>>; N],
}
