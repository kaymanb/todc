#[cfg(loom)]
pub(crate) use loom::sync::Mutex;
#[cfg(not(loom))]
pub(crate) use std::sync::Mutex;
