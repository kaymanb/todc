//! Simulations of [shared-memory registers](https://en.wikipedia.org/wiki/Shared_register).
//!
//! This module contains implementations of simulations of shared-memory
//! registers. These simulations are fault-tolerant, meaning that correctness
//! guarantees such as [atomicity](https://en.wikipedia.org/wiki/Atomic_semantics)
//! continue to hold even in the face of crashes and arbitrary message delays.
//!
//! # Examples
//!
//! See the [`abd_95`] module-level documentation for examples.
pub mod abd_95;

pub use self::abd_95::AtomicRegister;
