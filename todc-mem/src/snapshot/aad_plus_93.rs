//! Atomic snapshot objects, as described by Afek, Attiya,
//! Dolev, Gafni, Merritt and Shavit [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741).
//!
//! This module contains implementations of `N`-process single-writer multi-reader
//! snapshot objects. Both `BoundedSnapshot` and `UnboundedSnapshot` are abstracted
//! over a generic type `R: Register`, which is the type of primitive used to store
//! data. Many properties, such as wait-freedom and linearizability, depend on the
//! properties of the underlying register `R`.
//!
//! In general, if `R` is wait-free and linearizable, then all snapshot implementations
//! in this module will be as well. For more details on the differences between
//! register implementations, see `AtomicRegister`.
//!
//! # Restrictions on Atomic Snapshot Contents
//!
//! Due to restrictions on the number of bits of atomic shared-memory that is
//! available on most hardware (a maximum of 64), the `BoundedAtomicRegister`
//! and `UnboundedAtomicRegister` objects may only store values of type `u8`.
//! Similarily, the number `N` of components available in these snapshots is
//! limited to `6` and `5`, respectively.
//!
//! # Examples
//!
//! Obtain a consistent view of shared memory.
//!
//! ```
//! use std::sync::Arc;
//! use std::thread;
//! use todc_mem::snapshot::Snapshot;
//! use todc_mem::snapshot::aad_plus_93::BoundedAtomicSnapshot;
//!
//! const N: usize = 6;
//!
//! let snapshot: Arc<BoundedAtomicSnapshot<N>> = Arc::new(BoundedAtomicSnapshot::new());
//!
//! let mut handles = Vec::new();
//! for i in 1..N {
//!     let mut snapshot = snapshot.clone();
//!     handles.push(thread::spawn(move || {
//!         // Each thread update's it's component of the snapshot
//!         // to indicate that it has taken a step.
//!         snapshot.update(i, 1);
//!     }));
//! }
//!
//! snapshot.update(0, 1);
//!
//! // The main thread performs a scan, and filters the results
//! // to obtain a list of processes that had taken steps at
//! // at that instant.
//! let view = snapshot.scan(0);
//! let thread_ids: Vec<usize> = view
//!     .iter()
//!     .enumerate()
//!     .filter(|(i, &v)| v != 0)
//!     .map(|(i, v)| i + 1)
//!     .collect();
//!
//! println!("Threads {thread_ids:?} have taken steps!");
//!
//! for thread in handles {
//!     thread.join().unwrap();
//! }
//! ```
mod unbounded;
pub use unbounded::UnboundedAtomicSnapshot;
pub use unbounded::UnboundedMutexSnapshot;
pub use unbounded::UnboundedSnapshot;

mod bounded;
pub use bounded::BoundedAtomicSnapshot;
pub use bounded::BoundedMutexSnapshot;
pub use bounded::BoundedSnapshot;
