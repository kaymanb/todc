use super::common::{
    assert_random_operations_are_linearizable, NUM_ITERATIONS, NUM_OPERATIONS, NUM_PREEMPTIONS,
    NUM_THREADS,
};

mod lattice {
    use super::*;
    use todc_mem::snapshot::ar_98::LatticeMutexSnapshot;

    type MutexSnapshot = LatticeMutexSnapshot<u32, NUM_THREADS, 256>;

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
}
