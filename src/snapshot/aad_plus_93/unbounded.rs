use core::array::from_fn;
// use std::sync::atomic::{AtomicU64};

use crate::register::{AtomicRegister, Register};
use crate::snapshot::Snapshot;

#[derive(Clone, Copy)]
struct UnboundedContents<T: Copy + Default, const N: usize> {
    value: T,
    view: [T; N],
    sequence: u32,
}

impl<T: Copy + Default, const N: usize> Default for UnboundedContents<T, N> {
    fn default() -> Self {
        UnboundedContents {
            value: T::default(),
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
/// sequence numbers are stored as `u32`, and are unlikely to overflow
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
                return second.map(|c| c.value);
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

    fn update(&self, i: usize, value: Self::Value) {
        // Update the contents of the ith register with the
        // new value, an incremented sequence number, and the result
        // of a scan.
        let contents = UnboundedContents {
            value,
            sequence: self.registers[i].read().sequence + 1,
            view: self.scan(i),
        };
        self.registers[i].write(contents);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct UnboundedIntContents<const N: usize> {
    value: u8,
    view: [u8; N],
    sequence: u16,
}

impl<const N: usize> Default for UnboundedIntContents<N> {
    fn default() -> Self {
        // TODO: Find a better way to bound N
        if N > 5 {
            panic!("UnboundedIntContents are only valid for 5 threads or fewer")
        };
        Self {
            value: 0,
            view: [0; N],
            sequence: 0,
        }
    }
}

impl<const N: usize> From<u64> for UnboundedIntContents<N> {
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

impl<const N: usize> From<UnboundedIntContents<N>> for u64 {
    fn from(contents: UnboundedIntContents<N>) -> Self {
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

    mod unbounded_int_contents {
        use super::*;

        mod from_u64 {
            use super::*;

            #[test]
            fn decodes_if_two_threads() {
                let contents = UnboundedIntContents {
                    value: 200,
                    view: [1, 2],
                    sequence: 10_000,
                };
                let encoding: u64 = contents.clone().into();
                assert_eq!(contents, UnboundedIntContents::from(encoding));
            }

            #[test]
            fn decodes_if_three_threads() {
                let contents = UnboundedIntContents {
                    value: 200,
                    view: [1, 2, 3],
                    sequence: 10_000,
                };
                let encoding: u64 = contents.clone().into();
                assert_eq!(contents, UnboundedIntContents::from(encoding));
            }

            #[test]
            fn decodes_if_four_threads() {
                let contents = UnboundedIntContents {
                    value: 200,
                    view: [1, 2, 3, 4],
                    sequence: 10_000,
                };
                let encoding: u64 = contents.clone().into();
                assert_eq!(contents, UnboundedIntContents::from(encoding));
            }

            #[test]
            fn decodes_if_five_threads() {
                let contents = UnboundedIntContents {
                    value: 200,
                    view: [1, 2, 3, 4, 5],
                    sequence: 10_000,
                };
                let encoding: u64 = contents.clone().into();
                assert_eq!(contents, UnboundedIntContents::from(encoding));
            }
        }

        mod into_u64 {
            use super::*;

            macro_rules! encodes_default_as_zeros {
                ($($name:ident: $value:expr,)*) => {
                    $(
                        #[test]
                        fn $name() {
                            let actual: u64 = UnboundedIntContents::<$value>::default().into();
                            let expected: u64 = 0;
                            assert_eq!(actual, expected);
                        }
                    )*
                }
            }

            encodes_default_as_zeros! {
                default_zeroes_if_one_thread: 1,
                default_zeroes_if_two_threads: 2,
                default_zeroes_if_three_threads: 3,
                default_zeroes_if_four_threads: 4,
                default_zeroes_if_five_threads: 5,
            }

            #[test]
            fn encodes_if_two_threads() {
                let mut contents: UnboundedIntContents<2> = UnboundedIntContents::default();
                contents.value = 0b00100100;
                contents.view = [0b10000001, 0b10000000];
                contents.sequence = 0b11000000_11000000;
                let actual: u64 = contents.into();
                let expected: u64 =
                    0b00000000_00000000_00000000_11000000_11000000_10000000_10000001_00100100;
                assert_eq!(actual, expected);
            }

            #[test]
            fn encodes_if_three_threads() {
                let mut contents: UnboundedIntContents<3> = UnboundedIntContents::default();
                contents.value = 0b00100100;
                contents.view = [0b10000011, 0b10000001, 0b10000000];
                contents.sequence = 0b11000000_11000000;
                let actual: u64 = contents.into();
                let expected: u64 =
                    0b00000000_00000000_11000000_11000000_10000000_10000001_10000011_00100100;
                assert_eq!(actual, expected);
            }

            #[test]
            fn encodes_if_four_threads() {
                let mut contents: UnboundedIntContents<4> = UnboundedIntContents::default();
                contents.value = 0b00100100;
                contents.view = [0b10000111, 0b10000011, 0b10000001, 0b10000000];
                contents.sequence = 0b11000000_11000000;
                let actual: u64 = contents.into();
                let expected: u64 =
                    0b00000000_11000000_11000000_10000000_10000001_10000011_10000111_00100100;
                assert_eq!(actual, expected);
            }

            #[test]
            fn encodes_if_five_threads() {
                let mut contents: UnboundedIntContents<5> = UnboundedIntContents::default();
                contents.value = 0b00100100;
                contents.view = [0b10001111, 0b10000111, 0b10000011, 0b10000001, 0b10000000];
                contents.sequence = 0b11000000_11000000;
                let actual: u64 = contents.into();
                let expected: u64 =
                    0b11000000_11000000_10000000_10000001_10000011_10000111_10001111_00100100;
                assert_eq!(actual, expected);
            }
        }
    }
}
