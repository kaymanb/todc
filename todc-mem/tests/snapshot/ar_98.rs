use super::common::{
    assert_random_operations_are_linearizable, NUM_ITERATIONS, NUM_OPERATIONS, NUM_PREEMPTIONS,
    NUM_THREADS,
};

mod lattice {
    use super::*;
    use todc_mem::snapshot::LatticeMutexSnapshot;

    // Constant M must be a power of 2 and larger than NUM_OPERATIONS * NUM_THREADS
    type MutexSnapshot = LatticeMutexSnapshot<u32, NUM_THREADS, 512>;

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

    // This test executes a schedule that previously caused failures due to a
    // bug where the label being assigned to the root of the binary tree was
    // M, instead of the correct value M / 2. See the first paragraph of
    // "The Implementation:" on page 32 of the paper [AR98].
    #[cfg(feature = "shuttle")]
    #[test]
    fn mutex_snapshot_uses_root_label_of_m_over_two() {
        shuttle::replay_from_file(
            || {
                assert_random_operations_are_linearizable::<NUM_THREADS, MutexSnapshot>();
            },
            "tests/snapshot/replays/2023-09-16_lattice_atomic_snapshot_fails_linearization.log",
        );
    }
}
