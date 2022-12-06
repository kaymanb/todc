//! A shared snapshot object.

mod aad_plus;

pub use self::aad_plus::{BoundedAtomicSnapshot, UnboundedAtomicSnapshot};

/// A Snapshot object.
pub trait Snapshot {
    type Value;
    
    /// Creates a new n-component snapshot object, where each component
    /// is initialized to the input value. 
    fn new(n: u8, value: Self::Value) -> Self;
    
    /// Returns a vector containing the contents of the object..
    fn scan(&self) -> Vec<Self::Value>;
    
    /// Sets contents of the ith component to the specified value.
    fn update(&self, i: u8, value: Self::Value) -> ();

}
