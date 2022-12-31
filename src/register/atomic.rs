use std::sync::atomic::Ordering;

use ::atomic::Atomic;

use super::Register;

/// An atomic shared-memory register.
///
/// Native atomic instructions are used if they are available for `T`,
/// along with the strongest available memory ordering, Sequential Consistency.
///
/// Otherwise, the implementation falls-back to a spinlock based mechansim to
/// prevent concurrent access.
///
/// **Note:** Sequential consistency is slightly weaker than linearizability,
/// the synchronization condition usually associated with atomic memory.
/// In particular, sequentially consistent objects are not _composable_,
/// meaning that a program built of multiple sequentially consistent objects
/// might itself fail to be sequentially consistent.  
///
/// Fortunately, it has been shown that in asynchronous systems any program that
/// is linearizable when implemented from linearizable base objects is also
/// sequentially consistent when implemented from sequentially consistent base
/// objects [\[PPMG16\]](https://arxiv.org/abs/1607.06258). What this means is that,
/// for the purpose of implementing linearizable objects from atomic registers,
/// we are free to use sequentially consistent registers, like the one
/// implemented here, instead. The price we pay is that the implemented object
/// will also only be sequentially consistent.
#[derive(Debug)]
pub struct AtomicRegister<T: Copy + Default> {
    data: Atomic<T>,
    ordering: Ordering,
}

impl<T: Copy + Default> AtomicRegister<T> {
    /// Creates a new atomic register with specified initial value and
    /// memory ordering.
    fn new_with_order(ordering: Ordering) -> Self {
        Self {
            data: Atomic::new(T::default()),
            ordering: ordering,
        }
    }
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
        AtomicRegister::new_with_order(Ordering::SeqCst)
    }

    /// Returns the contents of the register.
    fn read(&self) -> Self::Value {
        self.data.load(self.ordering)
    }

    /// Sets the contents of the register.
    fn write(&self, value: Self::Value) -> () {
        self.data.store(value, self.ordering)
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
            AtomicRegister::<usize>::new();
        }

        #[test]
        fn test_read() {
            let register: AtomicRegister<usize> = AtomicRegister::new();
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

        const THING: Thing = Thing {
            color: Color::Red,
            height_in_ft: 5.9,
        };

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
