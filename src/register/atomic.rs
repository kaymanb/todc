use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

use super::Register;

// TODO: Explain nuance of SeqCst operations in an atomic context
pub struct AtomicRegister<T: Default + From<u64> + Into<u64>> {
    register: AtomicU64,
    _value_type: PhantomData<T>,
}

impl<T: Default + From<u64> + Into<u64>> Register for AtomicRegister<T> {
    type Value = T;

    fn new() -> Self {
        Self {
            register: AtomicU64::new(T::default().into()),
            _value_type: PhantomData,
        }
    }

    fn read(&self) -> T {
        self.register.load(Ordering::SeqCst).into()
    }

    fn write(&self, value: T) {
        self.register.store(value.into(), Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    struct Pair(bool, bool);

    impl From<Pair> for u64 {
        fn from(pair: Pair) -> Self {
            let mut result = u64::MAX;
            let Pair(first, second) = pair;
            result &= if first { 0b1 } else { 0b0 };
            result &= if second { 0b11 } else { 0b01 };
            result
        }
    }

    impl From<u64> for Pair {
        fn from(value: u64) -> Self {
            let first = value & 1 != 0;
            let second = (value & (1 << 1)) != 0;
            Pair(first, second)
        }
    }

    #[test]
    fn initializes_both_to_false() {
        let register: AtomicRegister<Pair> = AtomicRegister::new();
        assert_eq!(Pair(false, false), register.read());
    }

    #[test]
    fn read_returns_previously_written_value() {
        let pair = Pair(true, false);
        let register: AtomicRegister<Pair> = AtomicRegister::new();
        register.write(pair);
        assert_eq!(pair, register.read());
    }
}
