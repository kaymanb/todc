//! Implementations of atomic snapshot objects based on the paper by
//! Afek, Attiya, Dolev, Gafni, Merritt and Shavit [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741). 

use super::Snapshot;
use crate::register::{Register, AtomicRegister};

/// An atomic snapshot from unbounded single-writer multi-reader
/// atomic regisers. 
///
/// TODO: Explain why unbounded
pub struct UnboundedAtomicSnapshot { 
    
}

/// An atomic snapshot from single-writer multi-reader
/// atomic registers.
pub struct BoundedAtomicSnapshot {

}
