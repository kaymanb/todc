#[cfg(feature = "shuttle")]
pub(crate) use shuttle::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};
#[cfg(not(feature = "shuttle"))]
pub(crate) use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex,
};
