//! A shared read/write register.

mod atomic;

pub use self::atomic::AtomicRegister;

/// A shared-memory register.
pub trait Register: Clone {
    type Value;
    
    /// Creates a new register with specified initial value.
    fn new(value: Self::Value) -> Self;
    
    /// Returns the contents stored in the register.
    fn read(&self) -> Self::Value;
    
    /// Sets contents of the register to the specified value.
    fn write(&self, value: Self::Value) -> ();
}

