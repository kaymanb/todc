//! A shared snapshot object.

pub mod aad_plus_93;
pub mod ar_98;
pub mod mutex;

/// An N-component Snapshot object.
pub trait Snapshot<const N: usize> {
    type Value: Clone;

    /// Creates a snapshot object.
    fn new() -> Self;

    /// Returns an array containing the value of each component in the object.
    fn scan(&self, i: usize) -> [Self::Value; N];

    /// Sets contents of the ith component to the specified value.
    fn update(&self, i: usize, value: Self::Value);
}
