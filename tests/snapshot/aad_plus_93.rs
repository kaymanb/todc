use std::sync::Arc;

use loom::{sync::Mutex, thread};
use todc::linearizability::{WLGChecker, history::History};
use utils::specifications::snapshot::SnapshotSpecification;

use super::{RecordingSnapshot, TimedAction};

const NUM_THREADS: usize = 3;

mod unbounded_atomic_snapshot {
    use todc::snapshot::aad_plus_93::UnboundedAtomicSnapshot;

    use super::*;
    
    type ActionUnderTest = TimedAction<Option<usize>, NUM_THREADS>;
    type SnapshotUnderTest = RecordingSnapshot<NUM_THREADS, UnboundedAtomicSnapshot<Option<usize>, NUM_THREADS>>;


    #[test]
    fn test_one_shot_correctness() {
        loom::model(|| {
            let actions: Arc<Mutex<Vec<ActionUnderTest>>> = Arc::new(Mutex::new(vec![]));
            let mut handles = Vec::new();
            let snapshot: Arc<SnapshotUnderTest> = Arc::new(RecordingSnapshot::new());

            for i in 0..NUM_THREADS {
                let actions = actions.clone();
                let snapshot = snapshot.clone();
                handles.push(thread::spawn(move || {
                    let (update_call, update_resp) = snapshot.update(i, Some(i + 1));
                    let (scan_call, scan_resp) = snapshot.scan(i);
                    for action in [update_call, update_resp, scan_call, scan_resp] {
                        let mut actions = actions.lock().unwrap();
                        actions.push(action);
                    }
                }));
            }

            for handle in handles {
                handle.join().unwrap();
            }

            let actions = &mut *actions.lock().unwrap();
            actions.sort_by(|a, b| a.happened_at.cmp(&b.happened_at));
            let history = History::from_actions(actions.iter().map(|ta| (ta.process, ta.action.clone())).collect());
            // TODO: Check that all possible histories are being generated...
            assert!(WLGChecker::is_linearizable(SnapshotSpecification::init(), history));
        });
    }
}

mod bounded_atomic_snapshot {
    use super::*;
    use todc::snapshot::aad_plus_93::BoundedAtomicSnapshot;

    type ActionUnderTest = TimedAction<Option<usize>, NUM_THREADS>;
    type SnapshotUnderTest = RecordingSnapshot<NUM_THREADS, BoundedAtomicSnapshot<Option<usize>, NUM_THREADS>>;


    #[test]
    // TODO: Reduce code duplication between these tests.
    fn test_one_shot_correctness() {
        loom::model(|| {
            let actions: Arc<Mutex<Vec<ActionUnderTest>>> = Arc::new(Mutex::new(vec![]));
            let mut handles = Vec::new();
            let snapshot: Arc<SnapshotUnderTest> = Arc::new(RecordingSnapshot::new());

            for i in 0..NUM_THREADS {
                let actions = actions.clone();
                let snapshot = snapshot.clone();
                handles.push(thread::spawn(move || {
                    let (update_call, update_resp) = snapshot.update(i, Some(i + 1));
                    let (scan_call, scan_resp) = snapshot.scan(i);
                    for action in [update_call, update_resp, scan_call, scan_resp] {
                        let mut actions = actions.lock().unwrap();
                        actions.push(action);
                    }
                }));
            }

            for handle in handles {
                handle.join().unwrap();
            }

            let actions = &mut *actions.lock().unwrap();
            actions.sort_by(|a, b| a.happened_at.cmp(&b.happened_at));
            let history = History::from_actions(actions.iter().map(|ta| (ta.process, ta.action.clone())).collect());
            assert!(WLGChecker::is_linearizable(SnapshotSpecification::init(), history));
        });
    }
}
