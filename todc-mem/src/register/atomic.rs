use std::marker::PhantomData;

use crate::sync::{AtomicU64, Ordering};

use super::Register;

/// A shared-memory register, backed by an 64 bits of "atomic" memory.
///
/// This object works by serializing data and storing it in an
/// `AtomicU64`, and so can only be used to store small amounts
/// of data.
///
/// # Atomics and Memory Ordering
///
/// Unfortunately, the theoretical atomic memory model in which atomic really means
/// [_linearizable_](https://en.wikipedia.org/wiki/Linearizability),
/// is not the same as the model used by languages such as
/// [Rust](https://doc.rust-lang.org/nomicon/atomics.html) and
/// [C++](https://en.cppreference.com/w/cpp/atomic/memory_order).
/// In practice, the compiler and hardware optimizations that make life livable
/// come at the cost of potentially re-ordering memory accesses. The strictest
/// consistency model that we can ask for in Rust is
/// [_sequential consistency_](https://en.wikipedia.org/wiki/Sequential_consistency),
/// which means that all processes perform operations in a sequential order, but
/// the relative order of operations perfomed by different processes is undefined.
///
/// As a result , operations performed on an `AtomicRegister` are only
/// guaranteed to be sequentially consistent, not necessarily lineariazable.
///
/// Thankfully, it was recently shown by Perrin, Petrolia, Mostefaoui, and Jard
/// [[PPM+2016]](https://arxiv.org/abs/1607.06258) that objects that would become
/// linearizable if they were implemented on top of a linearizable memory become
/// sequentially consistent if implemented on top of sequentially consistent
/// memory. This means that, while implemenations of linearizable algorithms
/// from `AtomicRegister` objects may fail to be linearizable, they will at least
/// be sequentially consistent, and will retain all other properties such as
/// wait-freedom.
///
/// ## Linearizability
///
/// For a register that guarantees linearizability at the cost of lock-freedom,
/// see `MutexRegister`.
///
/// # Examples
///
/// A simple spinlock.
///
/// ```
/// use std::sync::Arc;
/// use std::{hint, thread};
/// use todc_mem::register::{AtomicRegister, Register};
///
///
/// let register: Arc<AtomicRegister<u64>> = Arc::new(AtomicRegister::new());
///
/// let register_clone = register.clone();
/// let thread = thread::spawn(move || {
///     register_clone.write(1)
/// });
///
/// while register.read() == 0 {
///     hint::spin_loop();
/// }
///
/// thread.join().unwrap();
/// ```
///
/// Although space is limited, it is still possible to store any type that can
/// be converted to `u64` and back again.
///
/// ```
/// use heapless::String;
/// use todc_mem::register::{AtomicRegister, Register};
///
/// // A String with a fixed capacity of 64 bits
/// #[derive(Clone, Debug, Default, PartialEq)]
/// struct TinyString(String<8>);
///
/// impl From<TinyString> for u64 {
///     fn from(string: TinyString) -> Self {
///         // -- snipped --
/// #       let mut result = Self::MAX;
/// #       let bytes = string.0.into_bytes();
/// #       for (i, num) in bytes.iter().rev().enumerate() {
/// #           let mut num = (*num as u64) << (i * 8);
/// #           for j in 0..i {
/// #               num |= (u8::MAX as u64) << (j * 8);
/// #           }
/// #           for k in 0..(8 - i - 1) {
/// #               num |= ((u8::MAX as u64) << ((8 - k - 1) * 8));
/// #           }
/// #           result &= num;
/// #       }
/// #       result
///     }
/// }
///
/// impl From<u64> for TinyString {
///     fn from(value: u64) -> Self {
///         // -- snipped --
/// #       let bytes: Vec<u8> = value.to_be_bytes()
/// #           .into_iter()
/// #           .filter(|&x| x != u8::MAX)
/// #           .collect();
/// #       let mut result: String<8> = String::from("");
/// #       if let Ok(string) = std::str::from_utf8(&bytes[..]) {
/// #           result.push_str(string);
/// #       };  
/// #       Self(result)
///     }
/// }
///
/// let register: AtomicRegister<TinyString> = AtomicRegister::new();
///
/// let empty = TinyString(String::from(""));
/// assert_eq!(register.read(), empty);
///
/// let greeting = TinyString(String::from("hi"));
/// register.write(greeting.clone());
/// assert_eq!(register.read(), greeting);
///
/// let emojis = TinyString(String::from("👋🦀"));
/// register.write(emojis.clone());
/// assert_eq!(register.read(), emojis);
/// ```
pub struct AtomicRegister<T: Default + From<u64> + Into<u64>> {
    register: AtomicU64,
    _value_type: PhantomData<T>,
}

impl<T: Default + From<u64> + Into<u64>> Register for AtomicRegister<T> {
    type Value = T;

    /// Creates a new register containing the default value of `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use todc_mem::register::{AtomicRegister, Register};
    ///
    /// let register: AtomicRegister<u64> = AtomicRegister::new();
    /// assert_eq!(register.read(), u64::default());
    /// ```
    fn new() -> Self {
        Self {
            register: AtomicU64::new(T::default().into()),
            _value_type: PhantomData,
        }
    }

    /// Returns the value currently contained in the register.
    ///
    /// # Examples
    ///
    /// ```
    /// use todc_mem::register::{AtomicRegister, Register};
    ///
    /// let register: AtomicRegister<u64> = AtomicRegister::new();
    /// assert_eq!(register.read(), 0);
    /// ```
    fn read(&self) -> T {
        self.register.load(Ordering::SeqCst).into()
    }

    /// Sets contents of the register to the specified value.
    ///
    /// # Examples
    ///
    /// ```
    /// use todc_mem::register::{AtomicRegister, Register};
    ///
    /// let register: AtomicRegister<u64> = AtomicRegister::new();
    /// register.write(42);
    /// assert_eq!(register.read(), 42);
    /// ```
    fn write(&self, value: T) {
        self.register.store(value.into(), Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    struct Pair(bool, bool);

    impl From<Pair> for u64 {
        fn from(pair: Pair) -> Self {
            let mut result = u64::MAX;
            let Pair(first, second) = pair;
            result &= if first { 0b1 } else { 0b0 };
            result &= if second { 0b11 } else { 0b01 };
            result
        }
    }

    impl From<u64> for Pair {
        fn from(value: u64) -> Self {
            let first = value & 1 != 0;
            let second = (value & (1 << 1)) != 0;
            Pair(first, second)
        }
    }

    #[test]
    fn initializes_both_to_false() {
        let register: AtomicRegister<Pair> = AtomicRegister::new();
        assert_eq!(Pair(false, false), register.read());
    }

    #[test]
    fn read_returns_previously_written_value() {
        let pair = Pair(true, false);
        let register: AtomicRegister<Pair> = AtomicRegister::new();
        register.write(pair);
        assert_eq!(pair, register.read());
    }
}
