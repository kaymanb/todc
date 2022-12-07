//! Implementations of atomic snapshot objects based on the paper by
//! Afek, Attiya, Dolev, Gafni, Merritt and Shavit [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741).
use core::array::from_fn;

use super::Snapshot;
use crate::register::{AtomicRegister, Register};

#[derive(Clone, Copy)]
struct UnboundedContents<T: Copy, const N: usize> {
    data: T,
    sequence: usize,
    view: [T; N],
}

/// An atomic snapshot from unbounded single-writer multi-reader
/// atomic regisers.
///
/// This implementation relies on storing sequence numbers that can
/// grow arbitrarily large, hence the dependence on _unbounded_
/// single-writer multi-writer atomic registers. In practice, these
/// sequence numbers are stored as `usize`, and are unlikely to overflow
/// during short-running programs.
pub struct UnboundedAtomicSnapshot<T: Copy, const N: usize> {
    registers: [AtomicRegister<UnboundedContents<T, N>>; N],
}

impl<T: Copy, const N: usize> UnboundedAtomicSnapshot<T, N> {
    fn collect(&self) -> [UnboundedContents<T, N>; N] {
        from_fn(|i| self.registers[i].read())
    }
}

impl<T: Copy, const N: usize> Snapshot<N> for UnboundedAtomicSnapshot<T, N> {
    type Value = T;

    fn new(value: Self::Value) -> Self {
        let initial_contents = UnboundedContents {
            data: value,
            sequence: 0,
            view: [value; N],
        };
        Self {
            registers: [(); N].map(|_| AtomicRegister::new(initial_contents)),
        }
    }

    fn scan(&self) -> [Self::Value; N] {
        // A process has moved if it it's sequence number
        // has been incremented.
        let mut moved = [0; N];
        loop {
            let first = self.collect();
            let second = self.collect();
            // If both collects are identical, then their values are a valid scan.
            if (0..N).all(|i| first[i].sequence == second[i].sequence) {
                return second.map(|c| c.data);
            }
            for j in 0..N {
                // If process j is observed to have moved twice, then it must 
                // have performed a succesfull update. The result of the scan
                // that it performed during that operation can be borrowed and
                // returned here.
                if first[j].sequence != second[j].sequence {
                    if moved[j] == 1 {
                        return second[j].view;
                    } else {
                        moved[j] += 1;
                    }
                }
            }
        }
    }

    fn update(&self, i: usize, value: Self::Value) -> () {
        // Update the contents of the ith register with the
        // new value, an incremented sequence number, and the result
        // of a scan.
        let contents = UnboundedContents {
            data: value,
            sequence: self.registers[i].read().sequence + 1,
            view: self.scan(),
        };
        self.registers[i].write(contents);
    }
}

/// An atomic snapshot from single-writer multi-reader
/// atomic registers.
pub struct BoundedAtomicSnapshot {}
