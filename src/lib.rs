use std::sync::atomic::{AtomicUsize, Ordering};

pub trait Register {
    type Value;

    fn new(value: Self::Value) -> Self;

    fn read(&self) -> Self::Value;

    fn write(&self, value: Self::Value) -> ();
}


pub struct IntegerRegister {
    data: AtomicUsize,
    ordering: Ordering
}

impl IntegerRegister {
    fn new_with_order(value: usize, ordering: Ordering) -> Self {
       Self {
           data: AtomicUsize::new(value), 
           ordering: ordering
       }
    }

}

impl Clone for IntegerRegister {
    fn clone(&self) -> IntegerRegister {
        IntegerRegister::new(self.read())
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
}
