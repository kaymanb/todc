use core::array::from_fn;

use crate::register::{MutexRegister, Register};
use crate::snapshot::Snapshot;

#[derive(Clone, Copy)]
struct BoundedContents<T: Copy + Default, const N: usize> {
    value: T,
    view: [T; N],
    // Handshake bits
    p: [bool; N],
    toggle: bool,
}

impl<T: Copy + Default, const N: usize> Default for BoundedContents<T, N> {
    fn default() -> Self {
        BoundedContents {
            value: T::default(),
            view: [T::default(); N],
            p: [false; N],
            toggle: false,
        }
    }
}

/// A single-writer atomic snapshot from single-writer multi-reader
/// atomic registers.
pub struct BoundedSnapshot<T: Copy + Default, const N: usize> {
    registers: [MutexRegister<BoundedContents<T, N>>; N],
    // Handshake bits
    q: [[MutexRegister<bool>; N]; N],
}

impl<T: Copy + Default, const N: usize> BoundedSnapshot<T, N> {
    fn collect(&self) -> [BoundedContents<T, N>; N] {
        from_fn(|i| self.registers[i].read())
    }
}

impl<T: Copy + Default, const N: usize> Snapshot<N> for BoundedSnapshot<T, N> {
    type Value = T;

    fn new() -> Self {
        Self {
            registers: [(); N].map(|_| MutexRegister::<BoundedContents<T, N>>::new()),
            q: [[(); N]; N].map(|arr| arr.map(|_| MutexRegister::<bool>::new())),
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
                return second.map(|c| c.value);
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

    fn update(&self, i: usize, value: Self::Value) {
        // Update the contents of the ith register with the new value, the
        // result of a scan, and negated handshake and toggle bits.
        let handshakes: [bool; N] = from_fn(|j| !self.q[j][i].read());
        let contents = BoundedContents {
            value,
            view: self.scan(i),
            p: handshakes,
            toggle: !self.registers[i].read().toggle,
        };
        self.registers[i].write(contents);
    }
}
