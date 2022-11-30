use std::sync::atomic::{AtomicU8, Ordering};

pub struct IntegerRegister {
    data: AtomicU8,
}

impl IntegerRegister {
    
    fn new(value: u8) -> Self {
        Self {
           data: AtomicU8::new(value) 
        }
    }

    fn read(&self) -> u8 {
        self.data.load(Ordering::Relaxed)
    }

    fn write(&self, value: u8) -> () {
        self.data.store(value, Ordering::Relaxed)
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
