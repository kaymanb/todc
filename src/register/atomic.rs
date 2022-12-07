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
pub struct AtomicRegister<T: Copy> {
    data: Atomic<T>,
    ordering: Ordering,
}

impl<T: Copy> AtomicRegister<T> {
    /// Creates a new atomic register with specified initial value and
    /// memory ordering.
    fn new_with_order(value: T, ordering: Ordering) -> Self {
        Self {
            data: Atomic::new(value),
            ordering: ordering,
        }
    }
}

impl<T: Copy> Register for AtomicRegister<T> {
    type Value = T;

    /// Creates a new atomic register with specified initial value.
    fn new(value: Self::Value) -> Self {
        AtomicRegister::new_with_order(value, Ordering::SeqCst)
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

impl<T: Copy> Clone for AtomicRegister<T> {
    fn clone(&self) -> AtomicRegister<T> {
        AtomicRegister::new(self.read())
    }
}

#[cfg(test)]
mod tests {
    use super::{AtomicRegister, Register};

    mod test_boolean {
        use super::{AtomicRegister, Register};

        #[test]
        fn test_new() {
            AtomicRegister::new(true);
        }

        #[test]
        fn test_read() {
            assert_eq!(true, AtomicRegister::new(true).read());
        }

        #[test]
        fn test_write() {
            let register = AtomicRegister::new(true);
            register.write(false);
            assert_eq!(false, register.read());
        }
    }

    mod test_integer {
        use super::{AtomicRegister, Register};

        #[test]
        fn test_new() {
            AtomicRegister::new(123);
        }

        #[test]
        fn test_read() {
            assert_eq!(123, AtomicRegister::new(123).read());
        }

        #[test]
        fn test_write() {
            let register = AtomicRegister::new(0);
            register.write(123);
            assert_eq!(123, register.read());
        }
    }

    mod test_struct {
        use super::{AtomicRegister, Register};

        #[derive(Clone, Copy, PartialEq, Debug)]
        enum Color {
            Red,
            Blue,
        }

        #[derive(Clone, Copy)]
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
            AtomicRegister::new(THING);
        }

        #[test]
        fn test_read() {
            let register = AtomicRegister::new(THING);
            let thing = register.read();
            let same_thing = Thing {
                color: Color::Red,
                height_in_ft: 5.9,
            };
            assert_eq!(thing.color, same_thing.color);
            assert_eq!(thing.height_in_ft, same_thing.height_in_ft);
        }

        #[test]
        fn test_write() {
            let register = AtomicRegister::new(THING);
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
