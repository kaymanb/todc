//! A sequential specification of a [register](https://en.wikipedia.org/wiki/Shared_register).
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::specifications::Specification;

/// An operation for a [register](https://en.wikipedia.org/wiki/Shared_register).
#[derive(Debug, Copy, Clone)]
pub enum RegisterOperation<T> {
    /// Read a value of type `T` from the register.
    ///
    /// If the return value of the operation is not-yet-known, then this can be
    /// represented as `Read(None)`.
    Read(Option<T>),
    /// Write a value of type `T` to the register.
    Write(T),
}

use RegisterOperation::*;

/// A sequential specification of a [register](https://en.wikipedia.org/wiki/Shared_register).
pub struct RegisterSpecification<T: Default + Eq> {
    data_type: PhantomData<T>,
}

impl<T: Clone + Debug + Default + Eq + Hash> Specification for RegisterSpecification<T> {
    type State = T;
    type Operation = RegisterOperation<T>;

    fn init() -> Self::State {
        T::default()
    }

    fn apply(operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
        match operation {
            Read(value) => {
                let value = value
                    .as_ref()
                    .expect("Cannot apply `Read` with unknown return value");
                (value == state, state.clone())
            }
            Write(value) => (true, value.clone()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    type Spec = RegisterSpecification<u32>;

    mod init {
        use super::*;

        #[test]
        fn initializes_state_to_default() {
            assert_eq!(Spec::init(), 0);
        }
    }

    mod apply {
        use super::*;

        #[test]
        fn read_is_valid_if_value_is_current_state() {
            let (is_valid, _) = Spec::apply(&Read(Some(0)), &Spec::init());
            assert!(is_valid);
        }

        #[test]
        fn read_is_not_valid_if_value_is_not_current_state() {
            let (is_valid, _) = Spec::apply(&Read(Some(1)), &Spec::init());
            assert!(!is_valid);
        }

        #[test]
        fn read_does_not_affect_state() {
            let old_state = Spec::init();
            let (_, new_state) = Spec::apply(&Read(Some(0)), &old_state);
            assert_eq!(old_state, new_state);
        }

        #[test]
        fn write_is_always_valid() {
            let (is_valid, _) = Spec::apply(&Write(1), &Spec::init());
            assert!(is_valid);
        }

        #[test]
        fn write_sets_new_state_to_written_value() {
            let value = 123;
            let (_, new_state) = Spec::apply(&Write(value), &Spec::init());
            assert_eq!(value, new_state);
        }
    }
}
