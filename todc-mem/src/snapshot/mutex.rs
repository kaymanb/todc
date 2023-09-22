//! An atomic snapshot backed by [`Mutex`] objects.
use crate::sync::Mutex;

use crate::snapshot::Snapshot;

/// A [`Mutex`]-based atomic snapshot.
///
/// This implementation uses a mutex to protect against concurrent memory
/// access. It is **not** lock-free.
pub struct MutexSnapshot<T: Copy + Default, const N: usize> {
    mutex: Mutex<[T; N]>,
}

impl<T: Copy + Default, const N: usize> Snapshot<N> for MutexSnapshot<T, N> {
    type Value = T;

    fn new() -> Self {
        Self {
            mutex: Mutex::new([(); N].map(|_| T::default())),
        }
    }

    /// Returns an array containing the value of each component in the object.
    fn scan(&self, _i: usize) -> [Self::Value; N] {
        *self.mutex.lock().unwrap()
    }

    /// Sets contents of the ith component to the specified value.
    fn update(&self, i: usize, value: Self::Value) {
        let mut data = self.mutex.lock().unwrap();
        data[i] = value;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn reads_and_writes() {
        let snapshot: MutexSnapshot<usize, 3> = MutexSnapshot::new();

        let view = snapshot.scan(0);
        assert_eq!(view, [0, 0, 0]);

        snapshot.update(1, 123);
        let view = snapshot.scan(1);
        assert_eq!(view, [0, 123, 0]);

        snapshot.update(2, 321);
        let view = snapshot.scan(2);
        assert_eq!(view, [0, 123, 321]);
    }
}
