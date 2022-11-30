use std::sync::atomic::{AtomicU8, Ordering};

pub struct IntegerRegister {
    data: AtomicU8,
    ordering: Ordering
}

impl IntegerRegister {
    
    fn new(value: u8) -> Self {
        IntegerRegister::new_with_order(value, Ordering::Relaxed)
    }

    fn new_with_order(value: u8, ordering: Ordering) -> Self {
       Self {
           data: AtomicU8::new(value), 
           ordering: ordering
       }
    }

    fn read(&self) -> u8 {
        self.data.load(self.ordering)
    }

    fn write(&self, value: u8) -> () {
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
