//! `N`-process snapshot objects.
//!
//! This module contains implementations of `N`-process snapshot objects. All
//! implementations are abstracted over a generic type `R: Register`, which is the
//! type of primitive used to store data. Many properties, such as wait-freedom and
//! linearizability, depend on the properties of the underlying register `R`.
//!
//! In general, if `R` is wait-free and linearizable, then all snapshot implementations
//! in this module will be as well. For more details on the differences between
//! register implementations, see [`AtomicRegister`](crate::register::AtomicRegister).
//!
//! # Restrictions on Atomic Snapshot Values
//!
//! Due to restrictions on the number of bits of atomic shared-memory that is
//! available on most hardware (a maximum of 64), the [`BoundedAtomicSnapshot`]
//! and [`UnboundedAtomicSnapshot`] objects may only store values of type [`u8`].
//! Similarily, the number `N` of components available in these snapshots is
//! limited to `6` and `5`, respectively.
//!
//! # Examples
//!
//! Obtain a consistent view of progress being made by a set of threads.
//!
//! ```
//! # use std::sync::atomic::{AtomicU8, Ordering};
//! use std::sync::Arc;
//! use std::thread;
//! use todc_mem::snapshot::{Snapshot, BoundedAtomicSnapshot};
//!
//! const N: usize = 6;
//!
//! let snapshot: Arc<BoundedAtomicSnapshot<N>> = Arc::new(BoundedAtomicSnapshot::new());
//!
//! # static hidden_state: [AtomicU8; N] = [
//! #   AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0),
//! #   AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)
//! # ];
//! fn do_work(i: usize) -> Option<u8> {
//!     // -- snipped --
//! #    let percent = hidden_state[i].load(Ordering::Acquire);
//! #    if percent < 100 {
//! #        hidden_state[i].store(percent + 1, Ordering::Release);
//! #        Some(percent)
//! #    } else {
//! #        None
//! #    }
//! }
//!
//! // Each worker thread does some work and periodically updates
//! // its component of the snapshot with the amount of progress
//! // it has made so far.
//! let mut handles = Vec::new();
//! for i in 1..N {
//!     let mut snapshot = snapshot.clone();
//!     handles.push(thread::spawn(move || {
//!         while let Some(percent_complete) = do_work(i) {
//!             snapshot.update(i, percent_complete);
//!         }        
//!     }));
//! }
//!
//! // The main thread waits until all workers have completed
//! // at least half of their work, before printing a message.
//! snapshot.update(0, 100);
//! loop {
//!     let view = snapshot.scan(0);
//!     if view.iter().all(|&p| p >= 50) {
//!         println!("We're at-least half-way done!");
//!         break;
//!     }
//! }
//!
//! for thread in handles {
//!     thread.join().unwrap();
//! }
//! ```
pub mod aad_plus_93;
pub mod ar_98;
pub mod mutex;

pub use self::aad_plus_93::{
    BoundedAtomicSnapshot, BoundedMutexSnapshot, UnboundedAtomicSnapshot, UnboundedMutexSnapshot,
};
pub use self::ar_98::LatticeMutexSnapshot;

/// An ID for a process (or thread).
pub type ProcessId = usize;

/// An `N`-component snapshot object.
pub trait Snapshot<const N: usize> {
    type Value: Clone;

    /// Creates a snapshot object.
    fn new() -> Self;

    /// Returns an array containing the value of each component in the object.
    fn scan(&self, i: ProcessId) -> [Self::Value; N];

    /// Sets contents of the _i^{th}_ component to the specified value.
    fn update(&self, i: ProcessId, value: Self::Value);
}
