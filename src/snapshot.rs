//! A shared snapshot object.

pub mod aad_plus;

/// An N-component Snapshot object.
pub trait Snapshot<const N: usize> {
    type Value;

    /// Creates a snapshot object where each component is set to the inital value.
    fn new(value: Self::Value) -> Self;

    /// Returns an array containing the value of each component in the object.
    fn scan(&self, i: usize) -> [Self::Value; N];

    /// Sets contents of the ith component to the specified value.
    fn update(&self, i: usize, value: Self::Value) -> ();
}
