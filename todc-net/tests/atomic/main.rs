use std::time::Instant;

use todc_utils::linearizability::history::Action;

#[path = "../common/mod.rs"]
mod common;
mod register;

type ProcessID = usize;

// TODO: Maybe put this into todc-utils?
pub struct TimedAction<T> {
    process: ProcessID,
    action: Action<T>,
    happened_at: Instant,
}

impl<T> TimedAction<T> {
    fn new(process: ProcessID, action: Action<T>) -> Self {
        Self {
            process,
            action,
            happened_at: Instant::now(),
        }
    }
}
