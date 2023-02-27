//! Implementations of atomic snapshot objects based on the paper by
//! Afek, Attiya, Dolev, Gafni, Merritt and Shavit [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741).
mod unbounded;
pub use unbounded::UnboundedSnapshot;

mod bounded;
pub use bounded::BoundedSnapshot;
