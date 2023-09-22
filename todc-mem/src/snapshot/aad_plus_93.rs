//! Snapshot objects, as described by by Afek, Attiya, Dolev, Gafni, Merritt
//! and Shavit [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741).
//!
//! # Examples
//! For examples, see the [`snapshot`](super) documentation.
mod unbounded;
pub use unbounded::UnboundedAtomicSnapshot;
pub use unbounded::UnboundedMutexSnapshot;
pub use unbounded::UnboundedSnapshot;

mod bounded;
pub use bounded::BoundedAtomicSnapshot;
pub use bounded::BoundedMutexSnapshot;
pub use bounded::BoundedSnapshot;
