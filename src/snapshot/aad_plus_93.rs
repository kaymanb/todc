//! Implementations of atomic snapshot objects based on the paper by
//! Afek, Attiya, Dolev, Gafni, Merritt and Shavit [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741).
use core::array::from_fn;

use super::Snapshot;
use crate::register::{AtomicRegister, Register};

#[derive(Clone, Copy)]
struct UnboundedContents<T: Copy + Default, const N: usize> {
    data: T,
    view: [T; N],
    sequence: usize,
}

impl<T: Copy + Default, const N: usize> Default for UnboundedContents<T, N> {
    fn default() -> Self {
        UnboundedContents {
            data: T::default(),
            view: [T::default(); N],
            sequence: 0,
        }
    }
}

/// An single-writer atomic snapshot from unbounded single-writer multi-reader
/// atomic regisers.
///
/// This implementation relies on storing sequence numbers that can
/// grow arbitrarily large, hence the dependence on _unbounded_
/// single-writer multi-writer atomic registers. In practice, these
/// sequence numbers are stored as `usize`, and are unlikely to overflow
/// during short-running programs.
pub struct UnboundedAtomicSnapshot<T: Copy + Default, const N: usize> {
    registers: [AtomicRegister<UnboundedContents<T, N>>; N],
}

impl<T: Copy + Default, const N: usize> UnboundedAtomicSnapshot<T, N> {
    fn collect(&self) -> [UnboundedContents<T, N>; N] {
        from_fn(|i| self.registers[i].read())
    }
}

impl<T: Copy + Default, const N: usize> Snapshot<N> for UnboundedAtomicSnapshot<T, N> {
    type Value = T;

    fn new() -> Self {
        Self {
            registers: [(); N].map(|_| AtomicRegister::<UnboundedContents<T, N>>::new()),
        }
    }

    fn scan(&self, _: usize) -> [Self::Value; N] {
        // A process has moved if it it's sequence number has been incremented.
        let mut moved = [0; N];
        loop {
            let first = self.collect();
            let second = self.collect();
            // If both collects are identical, then their values are a valid scan.
            if (0..N).all(|j| first[j].sequence == second[j].sequence) {
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
            view: self.scan(i),
        };
        self.registers[i].write(contents);
    }
}

#[derive(Clone, Copy)]
struct BoundedContents<T: Copy + Default, const N: usize> {
    data: T,
    view: [T; N],
    // Handshake bits
    p: [bool; N],
    toggle: bool,
}

impl<T: Copy + Default, const N: usize> Default for BoundedContents<T, N> {
    fn default() -> Self {
        BoundedContents {
            data: T::default(),
            view: [T::default(); N],
            p: [false; N],
            toggle: false,
        }
    }
}

/// A single-writer atomic snapshot from single-writer multi-reader
/// atomic registers.
pub struct BoundedAtomicSnapshot<T: Copy + Default, const N: usize> {
    registers: [AtomicRegister<BoundedContents<T, N>>; N],
    // Handshake bits
    q: [[AtomicRegister<bool>; N]; N],
}

impl<T: Copy + Default, const N: usize> BoundedAtomicSnapshot<T, N> {
    fn collect(&self) -> [BoundedContents<T, N>; N] {
        from_fn(|i| self.registers[i].read())
    }
}

impl<T: Copy + Default, const N: usize> Snapshot<N> for BoundedAtomicSnapshot<T, N> {
    type Value = T;

    fn new() -> Self {
        Self {
            registers: [(); N].map(|_| AtomicRegister::<BoundedContents<T, N>>::new()),
            q: [[(); N]; N].map(|arr| arr.map(|_| AtomicRegister::<bool>::new())),
        }
    }

    fn scan(&self, i: usize) -> [Self::Value; N] {
        // A process j has moved if its handshake bit p[i] differs from the corresponding
        // handshake bit and q[i][j] belonging to process i, _or_ if process j has
        // modified its toggle.
        let mut moved = [0; N];
        loop {
            // Collect handshake bits for all other processes
            for j in 0..N {
                self.q[i][j].write(self.registers[j].read().p[i]);
            }
            let first = self.collect();
            let second = self.collect();
            // If all handshake and toggle bits are equal then no process has moved, and hence no
            // process has performed an update during the double collect and we return can the result.
            if (0..N).all(|j| {
                let handshakes =
                    first[j].p[i] == second[j].p[i] && second[j].p[i] == self.q[i][j].read();
                let toggles = first[j].toggle == second[j].toggle;
                handshakes && toggles
            }) {
                return second.map(|c| c.data);
            }
            for j in 0..N {
                if first[j].p[i] != self.q[i][j].read()
                    || second[j].p[i] != self.q[i][j].read()
                    || first[j].toggle != second[j].toggle
                {
                    if moved[j] == 1 {
                        // If process j is observed to have moved twice, then it must
                        // have performed a succesfull update. The result of the scan
                        // that it performed during that operation can be borrowed and
                        // returned here.
                        return second[j].view;
                    } else {
                        moved[j] += 1;
                    }
                }
            }
        }
    }

    fn update(&self, i: usize, value: Self::Value) -> () {
        // Update the contents of the ith register with the new value, the
        // result of a scan, and negated handshake and toggle bits.
        let handshakes: [bool; N] = from_fn(|j| !self.q[j][i].read());
        let contents = BoundedContents {
            data: value,
            view: self.scan(i),
            p: handshakes,
            toggle: !self.registers[i].read().toggle,
        };
        self.registers[i].write(contents);
    }
}
