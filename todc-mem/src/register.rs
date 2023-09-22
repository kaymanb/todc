//! Shared read/write registers.
//!
//! See [`AtomicRegister`].
mod atomic;
pub use self::atomic::AtomicRegister;
mod mutex;
pub use self::mutex::MutexRegister;

/// A shared-memory register.
pub trait Register {
    type Value;

    /// Creates a new register.
    fn new() -> Self;

    /// Returns the value currently contained in the register.
    fn read(&self) -> Self::Value;

    /// Sets contents of the register to the specified value.
    fn write(&self, value: Self::Value);
}
