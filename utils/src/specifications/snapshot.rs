use core::array::from_fn;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use todc::linearizability::Specification;

use SnapshotOperation::{Scan, Update};

pub type ProcessID = usize;

#[derive(Debug, Copy, Clone)]
pub enum SnapshotOperation<T, const N: usize> {
    Scan(ProcessID, Option<[T; N]>),
    Update(ProcessID, T),
}

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
                None => panic!("Cannot apply scan without a resulting view"),
            },
            Update(i, value) => {
                let mut new_state = state.clone();
                new_state[*i] = value.clone();
                (true, new_state)
            }
        }
    }
}
