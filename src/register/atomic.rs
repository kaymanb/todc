use crate::sync::Mutex;

use super::Register;

/// An "atomic" shared-memory register.
///
/// This object uses a mutex to protect against concurrent memory access,
/// and is not lock-free.
#[derive(Debug)]
pub struct AtomicRegister<T: Copy + Default> {
    mutex: Mutex<T>,
}

impl<T: Copy + Default> Default for AtomicRegister<T> {
    fn default() -> Self {
        AtomicRegister::<T>::new()
    }
}

impl<T: Copy + Default> Register for AtomicRegister<T> {
    type Value = T;

    /// Creates a new atomic register with specified initial value.
    fn new() -> Self {
        AtomicRegister {
            mutex: Mutex::new(T::default()),
        }
    }

    /// Returns the contents of the register.
    fn read(&self) -> Self::Value {
        *self.mutex.lock().unwrap()
    }

    /// Sets the contents of the register.
    fn write(&self, value: Self::Value) -> () {
        *self.mutex.lock().unwrap() = value;
    }
}

impl<T: Copy + Default> Clone for AtomicRegister<T> {
    fn clone(&self) -> AtomicRegister<T> {
        let clone = AtomicRegister::new();
        clone.write(self.read());
        clone
    }
}

#[cfg(test)]
mod tests {
    use super::{AtomicRegister, Register};

    mod test_boolean {
        use super::{AtomicRegister, Register};

        #[test]
        fn test_new() {
            AtomicRegister::<bool>::new();
        }

        #[test]
        fn test_read() {
            let register: AtomicRegister<bool> = AtomicRegister::new();
            assert_eq!(false, register.read());
        }

        #[test]
        fn test_write() {
            let register = AtomicRegister::new();
            register.write(true);
            assert_eq!(true, register.read());
        }
    }

    mod test_integer {
        use super::{AtomicRegister, Register};

        #[test]
        fn test_new() {
            AtomicRegister::<u32>::new();
        }

        #[test]
        fn test_read() {
            let register: AtomicRegister<u32> = AtomicRegister::new();
            assert_eq!(0, register.read());
        }

        #[test]
        fn test_write() {
            let register = AtomicRegister::new();
            register.write(123);
            assert_eq!(123, register.read());
        }
    }

    mod test_struct {
        use super::{AtomicRegister, Register};

        #[derive(Clone, Copy, PartialEq, Debug, Default)]
        enum Color {
            #[default]
            Red,
            Blue,
        }

        #[derive(Clone, Copy, Default)]
        struct Thing {
            color: Color,
            height_in_ft: f32,
        }

        #[test]
        fn test_new() {
            AtomicRegister::<Thing>::new();
        }

        #[test]
        fn test_read() {
            let register: AtomicRegister<Thing> = AtomicRegister::new();
            let thing = register.read();
            let same_thing = Thing {
                color: Color::Red,
                height_in_ft: 0.0,
            };
            assert_eq!(thing.color, same_thing.color);
            assert_eq!(thing.height_in_ft, same_thing.height_in_ft);
        }

        #[test]
        fn test_write() {
            let register = AtomicRegister::new();
            let new_thing = Thing {
                color: Color::Blue,
                height_in_ft: 10.0,
            };
            register.write(new_thing);
            let contents = register.read();
            assert_eq!(contents.color, new_thing.color);
            assert_eq!(contents.height_in_ft, new_thing.height_in_ft);
        }
    }
}
