use std::sync::atomic::{AtomicUsize, Ordering};
use super::Register;

/// An atomic register storing integer values.
pub struct IntegerRegister {
    data: AtomicUsize,
    ordering: Ordering,
}

impl IntegerRegister {
    /// Creates a new atomic register with specified initial integer value and 
    /// memory ordering. 
    fn new_with_order(value: usize, ordering: Ordering) -> Self {
        Self {
            data: AtomicUsize::new(value),
            ordering: ordering,
        }
    }
}

impl Register for IntegerRegister {
    type Value = usize;
    
    /// Creates a new atomic register with specified initial integer value and
    /// the strongest available memory ordering, Sequential Consistency, 
    ///
    /// **Note:** Sequential consistency is slightly weaker than linearizability,
    /// the synchronization condition usually associated with atomic memory. 
    /// In particular, sequentially consistent objects are not _composable_, 
    /// meaning that a program built of multiple sequentially consistent objects 
    /// might itself fail to be sequentially consistent.  
    ///
    /// Fortunately, it has been shown that in asynchronous systems any program that
    /// is linearizable when implemented from linearizable base objects is also 
    /// sequentially consistent when implemented from sequentially consistent base 
    /// objects [PPMG16](https://arxiv.org/abs/1607.06258). What this means is that,
    /// for the purpose of implementing linearizable objects from atomic registers,
    /// we are free to use sequentially consistent registers, like the one 
    /// implemented here, instead. The price we pay is that the implemented object
    /// will also be sequentially consistent, instead of linearizable. 
    fn new(value: Self::Value) -> Self {
        IntegerRegister::new_with_order(value, Ordering::SeqCst)
    }

    fn read(&self) -> Self::Value {
        self.data.load(self.ordering)
    }

    fn write(&self, value: Self::Value) -> () {
        self.data.store(value, self.ordering)
    }
}

impl Clone for IntegerRegister {
    fn clone(&self) -> IntegerRegister {
        IntegerRegister::new(self.read())
    }
}

#[cfg(test)]
mod tests {
    use crate::register_tests;
    use super::IntegerRegister;

    register_tests! {
        test: IntegerRegister,
    }
}

