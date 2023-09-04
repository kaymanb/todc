//! Specifying the behavior of shared objects.
use std::fmt::Debug;
use std::hash::Hash;

pub mod etcd;
pub mod register;
pub mod snapshot;

/// A specification of a shared object.
///
/// This trait defines how operations performed on the object affect its state.
///
/// # Examples
///
/// For an example, see `RegisterSpecification`.
pub trait Specification {
    type State: Clone + Eq + Hash + Debug;
    type Operation: Clone + Debug;

    /// Returns an initial state for the object.
    fn init(&self) -> Self::State;

    /// Returns whether applying an operation to a given state is valid, and
    /// the new state that occurs after the operation has been applied.
    ///
    /// If the operation is not valid, then the state of the object should not change.
    fn apply(&self, op: &Self::Operation, state: &Self::State) -> (bool, Self::State);
}
