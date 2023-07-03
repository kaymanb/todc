//! Shared `N`-process snapshot objects.

pub mod aad_plus_93;
pub mod ar_98;
pub mod mutex;

/// An ID for a process (or thread).
pub type ProcessId = usize;

/// An `N`-component snapshot object.
pub trait Snapshot<const N: usize> {
    type Value: Clone;

    /// Creates a snapshot object.
    fn new() -> Self;

    /// Returns an array containing the value of each component in the object.
    fn scan(&self, i: ProcessId) -> [Self::Value; N];

    /// Sets contents of the _i^{th}_ component to the specified value.
    fn update(&self, i: ProcessId, value: Self::Value);
}
