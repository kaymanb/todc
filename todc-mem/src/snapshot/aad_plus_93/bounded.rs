use core::array::from_fn;
use std::fmt::Debug;

use crate::register::{AtomicRegister, MutexRegister, Register};
use crate::snapshot::Snapshot;
use crate::sync::{AtomicBool, Ordering};

/// A wait-free `N`-process atomic snapshot object, backed by [`AtomicRegister`]
/// objects.
///
/// Due to limitations on [`AtomicRegister`], this snapshot can only contain
/// `N <= 6` components of [`u8`] values. For implementation details, see
/// [`BoundedSnapshot`].
pub type BoundedAtomicSnapshot<const N: usize> =
    BoundedSnapshot<AtomicRegister<BoundedAtomicContents<N>>, N>;

/// An `N`-process atomic snapshot object, backed by [`MutexRegister`] objects.
///
/// This snapshot is **not** lock-free. For implementation details, see
/// [`BoundedSnapshot`].
pub type BoundedMutexSnapshot<T, const N: usize> =
    BoundedSnapshot<MutexRegister<BoundedContents<T, N>>, N>;

pub trait Contents<const N: usize>: Default {
    type Value: Copy + Debug;

    fn new(value: Self::Value, view: [Self::Value; N], handshakes: [bool; N], toggle: bool)
        -> Self;

    fn value(&self) -> Self::Value;

    fn view(&self) -> [Self::Value; N];

    fn handshake(&self, i: usize) -> bool;

    fn toggle(&self) -> bool;
}

/// A wait-free `N`-process snapshot object.
///
/// This implementation is described in Section 4 of
/// [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741), and builds on
/// the implementation of [`UnboundedSnapshot`](super::UnboundedSnapshot). If
/// the type of register `R` is linearizable, then [`BoundedSnapshot<R, N>`]
/// is as well.
pub struct BoundedSnapshot<R: Register, const N: usize>
where
    R::Value: Contents<N>,
{
    registers: [R; N],
    // The type for shared_handshakes could make use of another generic register
    // BoolR: Register where BoolR::Value: bool, but the additional generality
    // doesn't add much value when AtomicBool already exists.
    shared_handshakes: [[AtomicBool; N]; N],
}

impl<R: Register, const N: usize> BoundedSnapshot<R, N>
where
    R::Value: Contents<N>,
{
    fn collect(&self) -> [R::Value; N] {
        from_fn(|i| self.registers[i].read())
    }

    /// Returns whether process _i_ has seen process _j_ move while performing
    /// a double collect.
    ///
    /// A process _moves_ by changing their handshake bits when performing
    /// an update operation.
    fn has_moved(&self, first: &[R::Value; N], second: &[R::Value; N], i: usize, j: usize) -> bool {
        let first_changed =
            first[j].handshake(i) != self.shared_handshakes[i][j].load(Ordering::SeqCst);
        let second_changed =
            second[j].handshake(i) != self.shared_handshakes[i][j].load(Ordering::SeqCst);
        let toggle_changed = first[j].toggle() != second[j].toggle();
        first_changed || second_changed || toggle_changed
    }
}

impl<R: Register, const N: usize> Snapshot<N> for BoundedSnapshot<R, N>
where
    R::Value: Contents<N>,
{
    type Value = <R::Value as Contents<N>>::Value;

    fn new() -> Self {
        Self {
            registers: [(); N].map(|_| R::new()),
            shared_handshakes: [[(); N]; N].map(|arr| arr.map(|_| AtomicBool::new(false))),
        }
    }

    fn scan(&self, i: usize) -> [Self::Value; N] {
        let mut moved = [0; N];
        loop {
            // Collect handshake bits for all other processes
            for j in 0..N {
                let bit = self.registers[j].read().handshake(i);
                self.shared_handshakes[i][j].store(bit, Ordering::SeqCst);
            }
            let first = self.collect();
            let second = self.collect();
            // If all handshake and toggle bits are equal then no process has moved, and hence no
            // process has performed an update during the double collect and we return can the result.
            if (0..N).all(|j| !self.has_moved(&first, &second, i, j)) {
                return second.map(|c| c.value());
            }
            for j in 0..N {
                if self.has_moved(&first, &second, i, j) {
                    if moved[j] == 1 {
                        // If process j is observed to have moved twice, then it must
                        // have performed a succesfull update. The result of the scan
                        // that it performed during that operation can be borrowed and
                        // returned here.
                        return second[j].view();
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
        let view = self.scan(i);
        let toggle = !self.registers[i].read().toggle();
        let handshakes: [bool; N] =
            from_fn(|j| !self.shared_handshakes[j][i].load(Ordering::SeqCst));
        let contents = Contents::new(value, view, handshakes, toggle);
        self.registers[i].write(contents);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundedContents<T: Copy + Default, const N: usize> {
    value: T,
    view: [T; N],
    handshakes: [bool; N],
    toggle: bool,
}

impl<T: Copy + Default, const N: usize> Default for BoundedContents<T, N> {
    fn default() -> Self {
        Self {
            value: T::default(),
            view: [T::default(); N],
            handshakes: [bool::default(); N],
            toggle: bool::default(),
        }
    }
}

impl<T: Copy + Default + Debug, const N: usize> Contents<N> for BoundedContents<T, N> {
    type Value = T;

    fn new(
        value: Self::Value,
        view: [Self::Value; N],
        handshakes: [bool; N],
        toggle: bool,
    ) -> Self {
        Self {
            value,
            view,
            handshakes,
            toggle,
        }
    }

    fn value(&self) -> Self::Value {
        self.value
    }

    fn view(&self) -> [Self::Value; N] {
        self.view
    }

    fn handshake(&self, i: usize) -> bool {
        self.handshakes[i]
    }

    fn toggle(&self) -> bool {
        self.toggle
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BoundedAtomicContents<const N: usize> {
    // Occupies a total of 63 = 8 + (8*6) + (6*1) + 1 bits
    value: u8,
    view: [u8; N],
    handshakes: [bool; N],
    toggle: bool,
}

impl<const N: usize> Default for BoundedAtomicContents<N> {
    fn default() -> Self {
        // 6 process requires a total of 8 + (8*6) + (6*1) + 1 = 63 bits
        if N > 6 {
            panic!("BoundedAtomicContents are only valid for 6 threads or fewer")
        };
        Self {
            value: u8::default(),
            view: [u8::default(); N],
            handshakes: [bool::default(); N],
            toggle: bool::default(),
        }
    }
}

impl<const N: usize> Contents<N> for BoundedAtomicContents<N> {
    type Value = u8;

    fn new(
        value: Self::Value,
        view: [Self::Value; N],
        handshakes: [bool; N],
        toggle: bool,
    ) -> Self {
        Self {
            value,
            view,
            handshakes,
            toggle,
        }
    }

    fn value(&self) -> Self::Value {
        self.value
    }

    fn view(&self) -> [Self::Value; N] {
        self.view
    }

    fn handshake(&self, i: usize) -> bool {
        self.handshakes[i]
    }

    fn toggle(&self) -> bool {
        self.toggle
    }
}

impl<const N: usize> From<BoundedAtomicContents<N>> for u64 {
    fn from(contents: BoundedAtomicContents<N>) -> Self {
        let mut result: u64 = 0;
        // Encode value as right-most 8 bits
        result |= contents.value as u64;
        // Encode view as (reversed) sequence of 8-bit values
        for (i, value) in contents.view.iter().enumerate() {
            result |= (*value as u64) << (8 * (i + 1));
        }
        // Encode handshakes as (reversed) sequence of N bits
        for (i, boolean) in contents.handshakes.iter().enumerate() {
            result |= (*boolean as u64) << (8 * (N + 1) + i);
        }
        // Encode toggle as left-most bit.
        result |= (contents.toggle as u64) << 63;
        result
    }
}

impl<const N: usize> From<u64> for BoundedAtomicContents<N> {
    fn from(encoding: u64) -> Self {
        // Decode value from right-must 8 bits
        let value = (encoding & (u8::MAX as u64)) as u8;
        // Decode view from (reversed) sequence of 8-bit values
        let view = from_fn(|i| {
            let shift = 8 * (i + 1);
            ((encoding & (u8::MAX as u64) << shift) >> shift) as u8
        });
        // Decode handshakes from (reversed) sequence of N bits
        let handshakes = from_fn(|i| {
            let shift = 8 * (N + 1) + i;
            (encoding & 1 << shift) > 0
        });
        // Decode toggle from left-most bit.
        let toggle = (encoding & 1 << 63) > 0;
        Self {
            value,
            view,
            handshakes,
            toggle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod bounded_mutex_snapshot {
        use super::{BoundedMutexSnapshot, Snapshot};

        #[test]
        fn reads_and_writes() {
            let snapshot: BoundedMutexSnapshot<usize, 3> = BoundedMutexSnapshot::new();
            assert_eq!([0, 0, 0], snapshot.scan(0));
            snapshot.update(1, 11);
            snapshot.update(2, 12);
            assert_eq!([0, 11, 12], snapshot.scan(0));
        }
    }

    mod bounded_atomic_snapshot {
        use super::{BoundedAtomicSnapshot, Snapshot};

        #[test]
        fn reads_and_writes() {
            let snapshot: BoundedAtomicSnapshot<3> = BoundedAtomicSnapshot::new();
            assert_eq!([0, 0, 0], snapshot.scan(0));
            snapshot.update(1, 11);
            snapshot.update(2, 12);
            assert_eq!([0, 11, 12], snapshot.scan(0));
        }
    }

    mod bounded_atomic_contents {
        use super::BoundedAtomicContents;

        #[test]
        fn encodes_default_as_zeros() {
            let actual: u64 = BoundedAtomicContents::<6>::default().into();
            let expected: u64 = 0;
            assert_eq!(actual, expected);
        }

        #[test]
        fn decodes_zeroes_as_default() {
            let actual: BoundedAtomicContents<6> = 0.into();
            let expected: BoundedAtomicContents<6> = BoundedAtomicContents::default();
            assert_eq!(actual, expected);
        }

        #[test]
        fn encodes_to_u64_correctly() {
            let contents: BoundedAtomicContents<6> = BoundedAtomicContents::<6> {
                value: 0b00100100,
                view: [
                    0b10011111, 0b10001111, 0b10000111, 0b10000011, 0b10000001, 0b10000000,
                ],
                handshakes: [true, false, true, false, true, false],
                toggle: true,
            };
            let actual: u64 = contents.into();
            let expected: u64 =
                0b10010101_10000000_10000001_10000011_10000111_10001111_10011111_00100100;
            assert_eq!(actual, expected);
        }

        #[test]
        fn decodes_from_u64_correctly() {
            let contents = BoundedAtomicContents {
                value: 200,
                view: [1, 2, 3, 4, 5, 6],
                handshakes: [true, false, false, false, false, true],
                toggle: false,
            };
            let encoding: u64 = contents.into();
            assert_eq!(contents, BoundedAtomicContents::from(encoding));
        }
    }
}
