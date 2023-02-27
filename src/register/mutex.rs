use crate::sync::Mutex;

use super::Register;

/// An shared-memory register.
///
/// This object uses a mutex to protect against concurrent memory access,
/// and is not lock-free.
#[derive(Debug)]
pub struct MutexRegister<T: Copy + Default> {
    mutex: Mutex<T>,
}

impl<T: Copy + Default> Default for MutexRegister<T> {
    fn default() -> Self {
        MutexRegister::<T>::new()
    }
}

impl<T: Copy + Default> Register for MutexRegister<T> {
    type Value = T;

    /// Creates a new atomic register with specified initial value.
    fn new() -> Self {
        Self {
            mutex: Mutex::new(T::default()),
        }
    }

    /// Returns the contents of the register.
    fn read(&self) -> Self::Value {
        *self.mutex.lock().unwrap()
    }

    /// Sets the contents of the register.
    fn write(&self, value: Self::Value) {
        *self.mutex.lock().unwrap() = value;
    }
}

impl<T: Copy + Default> Clone for MutexRegister<T> {
    fn clone(&self) -> Self {
        let clone = Self::new();
        clone.write(self.read());
        clone
    }
}

#[cfg(test)]
mod tests {
    use super::{MutexRegister, Register};

    mod test_boolean {
        use super::{MutexRegister, Register};

        #[test]
        fn test_new() {
            MutexRegister::<bool>::new();
        }

        #[test]
        fn test_read() {
            let register: MutexRegister<bool> = MutexRegister::new();
            assert_eq!(false, register.read());
        }

        #[test]
        fn test_write() {
            let register = MutexRegister::new();
            register.write(true);
            assert_eq!(true, register.read());
        }
    }

    mod test_integer {
        use super::{MutexRegister, Register};

        #[test]
        fn test_new() {
            MutexRegister::<u32>::new();
        }

        #[test]
        fn test_read() {
            let register: MutexRegister<u32> = MutexRegister::new();
            assert_eq!(0, register.read());
        }

        #[test]
        fn test_write() {
            let register = MutexRegister::new();
            register.write(123);
            assert_eq!(123, register.read());
        }
    }

    mod test_struct {
        use super::{MutexRegister, Register};

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
            MutexRegister::<Thing>::new();
        }

        #[test]
        fn test_read() {
            let register: MutexRegister<Thing> = MutexRegister::new();
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
            let register = MutexRegister::new();
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
