//! Shared read/write registers.
mod atomic;
pub use self::atomic::AtomicRegister;
mod mutex;
pub use self::mutex::MutexRegister;

/// A shared-memory register.
pub trait Register {
    type Value;

    /// Creates a new register.
    fn new() -> Self;

    /// Returns the contents stored in the register.
    fn read(&self) -> Self::Value;

    /// Sets contents of the register to the specified value.
    fn write(&self, value: Self::Value);
}
