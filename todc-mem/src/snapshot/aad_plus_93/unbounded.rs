use core::array::from_fn;

use num::{One, PrimInt, Unsigned};

use crate::register::{AtomicRegister, MutexRegister, Register};
use crate::snapshot::Snapshot;

/// A wait-free `N`-process single-writer multi-reader atomic snapshot.
///
/// This implementation is backed by `AtomicRegister` objects.
// TODO: Mention limitations on N
pub type UnboundedAtomicSnapshot<const N: usize> =
    UnboundedSnapshot<AtomicRegister<UnboundedAtomicContents<N>>, N>;

/// An `N`-process single-writer multi-reader snapshot.
///
/// This implementation is backed by `MutexRegiser` objects,
/// and is linearizable but not lock-free.
pub type UnboundedMutexSnapshot<T, const N: usize> =
    UnboundedSnapshot<MutexRegister<UnboundedContents<T, N>>, N>;

/// The contents of a component of the snapshot object.
pub trait Contents<const N: usize>: Default {
    type Value: Copy;
    type SeqSize: PrimInt + Unsigned + One;

    /// Creates a new component.
    fn new(value: Self::Value, sequence: Self::SeqSize, view: [Self::Value; N]) -> Self;

    /// Returns the sequence number stored in this component.
    fn sequence(&self) -> Self::SeqSize;

    /// Returns the value stored in this component.
    fn value(&self) -> Self::Value;

    /// Returns the view stored in this component.
    fn view(&self) -> [Self::Value; N];
}

/// A wait-free `N`-process single-writer multi-reader snapshot object, backed by
/// register objects of type `R`.
///
/// This implementation relies on storing sequence numbers that can
/// grow arbitrarily large, and is described in Section 3 of
/// [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741). If `R`
/// is linearizable, then `UnboundedSnapshot<R, N>` is as well.
pub struct UnboundedSnapshot<R: Register, const N: usize>
where
    R::Value: Contents<N>,
{
    registers: [R; N],
}

impl<R: Register, const N: usize> UnboundedSnapshot<R, N>
where
    R::Value: Contents<N>,
{
    /// Returns an array of values, obtained by sequentially
    /// performing a read on each component of the snapshot.
    fn collect(&self) -> [R::Value; N] {
        from_fn(|i| self.registers[i].read())
    }
}

impl<R: Register, const N: usize> Snapshot<N> for UnboundedSnapshot<R, N>
where
    R::Value: Contents<N>,
{
    type Value = <R::Value as Contents<N>>::Value;

    /// Creates a new snapshot object.
    fn new() -> Self {
        Self {
            registers: [(); N].map(|_| R::new()),
        }
    }

    fn scan(&self, _: usize) -> [Self::Value; N] {
        // A process has moved if it it's sequence number has been incremented.
        let mut moved = [0; N];
        loop {
            let first = self.collect();
            let second = self.collect();
            // If both collects are identical, then their values are a valid scan.
            if (0..N).all(|j| first[j].sequence() == second[j].sequence()) {
                return second.map(|c| c.value());
            }
            for j in 0..N {
                // If process j is observed to have moved twice, then it must
                // have performed a succesfull update. The result of the scan
                // that it performed during that operation can be borrowed and
                // returned here.
                if first[j].sequence() != second[j].sequence() {
                    if moved[j] == 1 {
                        return second[j].view();
                    } else {
                        moved[j] += 1;
                    }
                }
            }
        }
    }

    fn update(&self, i: usize, value: Self::Value) {
        // Update the contents of the ith register with the
        // new value, an incremented sequence number, and the result
        // of a scan.
        let contents = Contents::new(
            value,
            self.registers[i].read().sequence() + <R::Value as Contents<N>>::SeqSize::one(),
            self.scan(i),
        );
        self.registers[i].write(contents);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnboundedContents<T: Copy + Default, const N: usize> {
    value: T,
    view: [T; N],
    sequence: u32,
}

impl<T: Copy + Default, const N: usize> Default for UnboundedContents<T, N> {
    fn default() -> Self {
        Self {
            value: T::default(),
            view: [T::default(); N],
            sequence: 0,
        }
    }
}

impl<T: Copy + Default, const N: usize> Contents<N> for UnboundedContents<T, N> {
    type Value = T;
    type SeqSize = u32;

    fn new(value: Self::Value, sequence: Self::SeqSize, view: [Self::Value; N]) -> Self {
        Self {
            value,
            view,
            sequence,
        }
    }

    fn value(&self) -> Self::Value {
        self.value
    }

    fn view(&self) -> [Self::Value; N] {
        self.view
    }

    fn sequence(&self) -> Self::SeqSize {
        self.sequence
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct UnboundedAtomicContents<const N: usize> {
    value: u8,
    view: [u8; N],
    sequence: u16,
}

impl<const N: usize> Contents<N> for UnboundedAtomicContents<N> {
    type Value = u8;
    type SeqSize = u16;

    fn new(value: Self::Value, sequence: Self::SeqSize, view: [Self::Value; N]) -> Self {
        Self {
            value,
            view,
            sequence,
        }
    }

    fn value(&self) -> Self::Value {
        self.value
    }

    fn view(&self) -> [Self::Value; N] {
        self.view
    }

    fn sequence(&self) -> Self::SeqSize {
        self.sequence
    }
}

impl<const N: usize> Default for UnboundedAtomicContents<N> {
    fn default() -> Self {
        // TODO: Find a better way to bound N
        if N > 5 {
            panic!("UnboundedAtomicContents are only valid for 5 threads or fewer")
        };
        Self {
            value: 0,
            view: [0; N],
            sequence: 0,
        }
    }
}

impl<const N: usize> From<u64> for UnboundedAtomicContents<N> {
    fn from(encoding: u64) -> Self {
        // Decode value from right-must 8 bits
        let value = (encoding & (u8::MAX as u64)) as u8;
        // Decode view from (reversed) sequence of 8-bit values
        let view = from_fn(|i| {
            let shift = 8 * (i + 1);
            ((encoding & (u8::MAX as u64) << shift) >> shift) as u8
        });
        // Decode sequence number from remaining left-most bits
        let shift = 8 * (N + 1);
        let sequence = ((encoding & ((u16::MAX as u64) << shift)) >> shift) as u16;
        Self {
            value,
            view,
            sequence,
        }
    }
}

impl<const N: usize> From<UnboundedAtomicContents<N>> for u64 {
    fn from(contents: UnboundedAtomicContents<N>) -> Self {
        let mut result: u64 = 0;
        // Encode value as right-most 8 bits
        result |= contents.value as u64;
        // Encode view as (reversed) sequence of 8-bit values
        for (i, value) in contents.view.iter().enumerate() {
            result |= (*value as u64) << (8 * (i + 1))
        }
        // Encode sequence number in remaining left-most bits
        result |= (contents.sequence as u64) << (8 * (N + 1));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod unbounded_mutex_snapshot {
        use super::*;

        #[test]
        fn reads_and_writes() {
            let snapshot: UnboundedMutexSnapshot<usize, 3> = UnboundedMutexSnapshot::new();
            assert_eq!([0, 0, 0], snapshot.scan(0));
            snapshot.update(1, 11);
            snapshot.update(2, 12);
            assert_eq!([0, 11, 12], snapshot.scan(0));
        }
    }

    mod unbounded_atomic_snapshot {
        use super::*;

        #[test]
        fn reads_and_writes() {
            let snapshot: UnboundedAtomicSnapshot<3> = UnboundedAtomicSnapshot::new();
            assert_eq!([0, 0, 0], snapshot.scan(0));
            snapshot.update(1, 11);
            snapshot.update(2, 12);
            assert_eq!([0, 11, 12], snapshot.scan(0));
        }
    }

    mod unbounded_int_contents {
        use super::*;

        mod from_u64 {
            use super::*;

            #[test]
            fn decodes_if_two_processes() {
                let contents = UnboundedAtomicContents {
                    value: 200,
                    view: [1, 2],
                    sequence: 10_000,
                };
                let encoding: u64 = contents.into();
                assert_eq!(contents, UnboundedAtomicContents::from(encoding));
            }

            #[test]
            fn decodes_if_three_processes() {
                let contents = UnboundedAtomicContents {
                    value: 200,
                    view: [1, 2, 3],
                    sequence: 10_000,
                };
                let encoding: u64 = contents.into();
                assert_eq!(contents, UnboundedAtomicContents::from(encoding));
            }

            #[test]
            fn decodes_if_four_processes() {
                let contents = UnboundedAtomicContents {
                    value: 200,
                    view: [1, 2, 3, 4],
                    sequence: 10_000,
                };
                let encoding: u64 = contents.into();
                assert_eq!(contents, UnboundedAtomicContents::from(encoding));
            }

            #[test]
            fn decodes_if_five_processes() {
                let contents = UnboundedAtomicContents {
                    value: 200,
                    view: [1, 2, 3, 4, 5],
                    sequence: 10_000,
                };
                let encoding: u64 = contents.into();
                assert_eq!(contents, UnboundedAtomicContents::from(encoding));
            }
        }

        mod into_u64 {
            use super::*;

            macro_rules! encodes_default_as_zeros {
                ($($name:ident: $value:expr,)*) => {
                    $(
                        #[test]
                        fn $name() {
                            let actual: u64 = UnboundedAtomicContents::<$value>::default().into();
                            let expected: u64 = 0;
                            assert_eq!(actual, expected);
                        }
                    )*
                }
            }

            encodes_default_as_zeros! {
                default_zeroes_if_one_thread: 1,
                default_zeroes_if_two_processes: 2,
                default_zeroes_if_three_processes: 3,
                default_zeroes_if_four_processes: 4,
                default_zeroes_if_five_processes: 5,
            }

            #[test]
            fn encodes_if_two_processes() {
                let contents: UnboundedAtomicContents<2> = UnboundedAtomicContents {
                    value: 0b00100100,
                    view: [0b10000001, 0b10000000],
                    sequence: 0b11000000_11000000,
                };
                let actual: u64 = contents.into();
                let expected: u64 =
                    0b00000000_00000000_00000000_11000000_11000000_10000000_10000001_00100100;
                assert_eq!(actual, expected);
            }

            #[test]
            fn encodes_if_three_processes() {
                let contents: UnboundedAtomicContents<3> = UnboundedAtomicContents {
                    value: 0b00100100,
                    view: [0b10000011, 0b10000001, 0b10000000],
                    sequence: 0b11000000_11000000,
                };
                let actual: u64 = contents.into();
                let expected: u64 =
                    0b00000000_00000000_11000000_11000000_10000000_10000001_10000011_00100100;
                assert_eq!(actual, expected);
            }

            #[test]
            fn encodes_if_four_processes() {
                let contents: UnboundedAtomicContents<4> = UnboundedAtomicContents {
                    value: 0b00100100,
                    view: [0b10000111, 0b10000011, 0b10000001, 0b10000000],
                    sequence: 0b11000000_11000000,
                };
                let actual: u64 = contents.into();
                let expected: u64 =
                    0b00000000_11000000_11000000_10000000_10000001_10000011_10000111_00100100;
                assert_eq!(actual, expected);
            }

            #[test]
            fn encodes_if_five_processes() {
                let contents: UnboundedAtomicContents<5> = UnboundedAtomicContents {
                    value: 0b00100100,
                    view: [0b10001111, 0b10000111, 0b10000011, 0b10000001, 0b10000000],
                    sequence: 0b11000000_11000000,
                };
                let actual: u64 = contents.into();
                let expected: u64 =
                    0b11000000_11000000_10000000_10000001_10000011_10000111_10001111_00100100;
                assert_eq!(actual, expected);
            }
        }
    }
}
