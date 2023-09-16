//! A sequential specification of a [snapshot object](https://en.wikipedia.org/wiki/Shared_snapshot_objects).
use core::array::from_fn;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use crate::specifications::Specification;

use SnapshotOperation::{Scan, Update};

/// A process identifier.
pub type ProcessId = usize;

/// An operation for a snapshot object.
#[derive(Debug, Copy, Clone)]
pub enum SnapshotOperation<T, const N: usize> {
    /// Scan the object and return an view containing the values in each component.
    ///
    /// If the return value of a scan is not-yet-known, this can be represented
    /// as `Scan(pid, None)`.
    Scan(ProcessId, Option<[T; N]>),
    /// Update the component of the object belonging to this process.
    Update(ProcessId, T),
}

/// A specification of an `N`-process [snapshot object](https://en.wikipedia.org/wiki/Shared_snapshot_objects).
///
/// Each component of the snapshot contains a value of type `T`.
pub struct SnapshotSpecification<T: Clone + Debug + Default + Eq + Hash, const N: usize> {
    data_type: PhantomData<T>,
}

impl<T: Clone + Debug + Default + Eq + Hash, const N: usize> SnapshotSpecification<T, N> {
    // Required so that the phantom field data_type can be instantiated.
    pub fn init() -> Self {
        Self {
            data_type: PhantomData,
        }
    }
}

impl<T: Clone + Debug + Default + Eq + Hash, const N: usize> Specification
    for SnapshotSpecification<T, N>
{
    type State = [T; N];
    type Operation = SnapshotOperation<T, N>;

    fn init(&self) -> Self::State {
        from_fn(|_| T::default())
    }

    fn apply(&self, operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
        match operation {
            Scan(_, result) => match result {
                Some(view) => (view == state, state.clone()),
                None => panic!("Cannot apply Scan with an unknown return value."),
            },
            Update(i, value) => {
                let mut new_state = state.clone();
                new_state[*i] = value.clone();
                (true, new_state)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SnapshotOperation::*, SnapshotSpecification, Specification};

    mod init {
        use super::*;

        #[test]
        fn returns_array_of_defaults() {
            let spec = SnapshotSpecification::<u32, 3>::init();
            let state = spec.init();
            assert_eq!(state, [u32::default(), u32::default(), u32::default()]);
        }
    }

    mod apply {
        use super::*;

        type Value = u32;
        const NUM_PROCESSES: usize = 3;

        fn setup() -> (
            SnapshotSpecification<Value, NUM_PROCESSES>,
            [Value; NUM_PROCESSES],
        ) {
            let spec = SnapshotSpecification::<Value, NUM_PROCESSES>::init();
            let state = spec.init();
            (spec, state)
        }

        #[test]
        fn update_applied_to_proper_component() {
            let id = 1;
            let value = 123;
            let (spec, state) = setup();
            let (_, new_state) = spec.apply(&Update(id, value), &state);
            assert_eq!(new_state[id], value);
            for i in [0, 2] {
                assert_eq!(new_state[i], Value::default());
            }
        }

        #[test]
        fn update_always_valid() {
            let (spec, state) = setup();
            for i in 0..NUM_PROCESSES {
                let (valid, _) = spec.apply(&Update(i, i as u32), &state);
                assert!(valid);
            }
        }

        #[test]
        fn scan_doesnt_affect_state() {
            let (spec, state) = setup();
            let (_, new_state) = spec.apply(&Scan(0, Some([0, 0, 0])), &state);
            assert_eq!(state, new_state);
        }

        #[test]
        fn scan_not_valid_if_differs_from_state() {
            let (spec, state) = setup();
            let mut new_state = state.clone();
            new_state[0] = 123;
            let (valid, _) = spec.apply(&Scan(0, Some(new_state)), &state);
            assert!(!valid);
        }
    }
}
