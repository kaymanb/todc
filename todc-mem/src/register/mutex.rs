use crate::sync::Mutex;

use super::Register;

/// An shared-memory register, backed by a [`Mutex`].
///
/// This object uses a mutex to protect against concurrent memory
/// access. It is **not** lock-free.
///
/// # Examples
///
/// A simple spinlock.
///
/// ```
/// use std::sync::Arc;
/// use std::{hint, thread};
/// use todc_mem::register::{MutexRegister, Register};
///
///
/// let register: Arc<MutexRegister<bool>> = Arc::new(MutexRegister::new());
///
/// let register_clone = register.clone();
/// let thread = thread::spawn(move || {
///     register_clone.write(true)
/// });
///
/// while !register.read() {
///     hint::spin_loop();
/// }
///
/// thread.join().unwrap();
/// ```
///
/// It is also possible to store larger, more complicated objects.
///
/// ```
/// use todc_mem::register::{MutexRegister, Register};
///
/// #[derive(Clone, Copy, Debug, Default, PartialEq)]
/// enum MyType {
///     #[default]
///     Nothing,
///     Booleans([bool; 100]),
///     Numbers([u64; 100]),
/// }
///
/// let register: MutexRegister<MyType> = MutexRegister::new();
///
/// assert_eq!(register.read(), MyType::Nothing);
///
/// let numbers = MyType::Numbers([42; 100]);
/// register.write(numbers);
/// assert_eq!(register.read(), numbers);
/// ```
///
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

    /// Creates a new register containing the default value of `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use todc_mem::register::{MutexRegister, Register};
    ///
    /// let register: MutexRegister<bool> = MutexRegister::new();
    /// assert_eq!(register.read(), bool::default());
    /// ```
    fn new() -> Self {
        Self {
            mutex: Mutex::new(T::default()),
        }
    }

    /// Returns the value currently contained in the register.
    ///
    /// # Examples
    ///
    /// ```
    /// use todc_mem::register::{MutexRegister, Register};
    ///
    /// let register: MutexRegister<bool> = MutexRegister::new();
    /// assert_eq!(register.read(), false);
    /// ```
    fn read(&self) -> Self::Value {
        *self.mutex.lock().unwrap()
    }

    /// Sets contents of the register to the specified value.
    ///
    /// # Examples
    ///
    /// ```
    /// use todc_mem::register::{MutexRegister, Register};
    ///
    /// let register: MutexRegister<bool> = MutexRegister::new();
    /// register.write(true);
    /// assert_eq!(register.read(), true);
    /// ```
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

    mod boolean {
        use super::{MutexRegister, Register};

        #[test]
        fn new() {
            MutexRegister::<bool>::new();
        }

        #[test]
        fn read() {
            let register: MutexRegister<bool> = MutexRegister::new();
            assert!(!register.read());
        }

        #[test]
        fn write() {
            let register = MutexRegister::new();
            register.write(true);
            assert!(register.read());
        }
    }

    mod integer {
        use super::{MutexRegister, Register};

        #[test]
        fn new() {
            MutexRegister::<u32>::new();
        }

        #[test]
        fn read() {
            let register: MutexRegister<u32> = MutexRegister::new();
            assert_eq!(0, register.read());
        }

        #[test]
        fn write() {
            let register = MutexRegister::new();
            register.write(123);
            assert_eq!(123, register.read());
        }
    }

    mod custom_struct {
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
        fn new() {
            MutexRegister::<Thing>::new();
        }

        #[test]
        fn read() {
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
        fn write() {
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
