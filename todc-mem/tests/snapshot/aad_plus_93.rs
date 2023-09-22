use super::common::{
    assert_random_operations_are_linearizable, NUM_ITERATIONS, NUM_PREEMPTIONS, NUM_THREADS,
};

mod unbounded {
    use super::*;
    use todc_mem::snapshot::{UnboundedAtomicSnapshot, UnboundedMutexSnapshot};

    type MutexSnapshot = UnboundedMutexSnapshot<u32, NUM_THREADS>;
    type AtomicSnapshot = UnboundedAtomicSnapshot<NUM_THREADS>;

    #[cfg(feature = "shuttle")]
    #[test]
    fn mutex_snapshot_is_linearizable() {
        shuttle::check_pct(
            || {
                assert_random_operations_are_linearizable::<NUM_THREADS, MutexSnapshot>();
            },
            NUM_ITERATIONS,
            NUM_PREEMPTIONS,
        );
    }

    #[cfg(feature = "shuttle")]
    #[test]
    fn atomic_snapshot_is_linearizable() {
        shuttle::check_pct(
            || {
                assert_random_operations_are_linearizable::<NUM_THREADS, AtomicSnapshot>();
            },
            NUM_ITERATIONS,
            NUM_PREEMPTIONS,
        );
    }
}

mod bounded {
    use super::*;
    use todc_mem::snapshot::{BoundedAtomicSnapshot, BoundedMutexSnapshot};

    type MutexSnapshot = BoundedMutexSnapshot<u32, NUM_THREADS>;
    type AtomicSnapshot = BoundedAtomicSnapshot<NUM_THREADS>;

    #[cfg(feature = "shuttle")]
    #[test]
    fn mutex_snapshot_is_linearizable() {
        shuttle::check_pct(
            || {
                assert_random_operations_are_linearizable::<NUM_THREADS, MutexSnapshot>();
            },
            NUM_ITERATIONS,
            NUM_PREEMPTIONS,
        );
    }

    #[cfg(feature = "shuttle")]
    #[test]
    fn atomic_snapshot_is_linearizable() {
        shuttle::check_pct(
            || {
                assert_random_operations_are_linearizable::<NUM_THREADS, AtomicSnapshot>();
            },
            NUM_ITERATIONS,
            NUM_PREEMPTIONS,
        );
    }
}
