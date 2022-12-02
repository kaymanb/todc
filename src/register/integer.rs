use std::sync::atomic::{AtomicUsize, Ordering};
use super::Register;

pub struct IntegerRegister {
    data: AtomicUsize,
    ordering: Ordering,
}

impl IntegerRegister {
    fn new_with_order(value: usize, ordering: Ordering) -> Self {
        Self {
            data: AtomicUsize::new(value),
            ordering: ordering,
        }
    }
}

impl Register for IntegerRegister {
    type Value = usize;

    fn new(value: Self::Value) -> Self {
        IntegerRegister::new_with_order(value, Ordering::SeqCst)
    }

    fn read(&self) -> Self::Value {
        self.data.load(self.ordering)
    }

    fn write(&self, value: Self::Value) -> () {
        self.data.store(value, self.ordering)
    }
}

impl Clone for IntegerRegister {
    fn clone(&self) -> IntegerRegister {
        IntegerRegister::new(self.read())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        IntegerRegister::new(0);
    }

    #[test]
    fn test_read() {
        let register = IntegerRegister::new(0);
        assert_eq!(0, register.read());
    }

    #[test]
    fn test_write() {
        let register = IntegerRegister::new(0);
        register.write(1);
        assert_eq!(1, register.read());
    }

    #[test]
    fn test_clone() {
        let register = IntegerRegister::new(1);
        assert_eq!(register.read(), register.clone().read());
    }
}
