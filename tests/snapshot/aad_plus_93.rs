use std::sync::Arc;
use loom::{thread, sync::Mutex};
use todc::snapshot::Snapshot;

use super::common;

const NUM_THREADS: usize = 3;

mod unbounded_atomic_snapshot {
    use super::*;
    use todc::snapshot::aad_plus_93::UnboundedAtomicSnapshot;

    #[test]
    fn test_one_shot_correctness() {
        loom::model(|| {
            let results = Arc::new(Mutex::new(vec![]));
            let mut handles = vec![];
            let snapshot: Arc<UnboundedAtomicSnapshot<Option<usize>, NUM_THREADS>> = Arc::new(UnboundedAtomicSnapshot::new());

            for i in 0..NUM_THREADS {
                let results = Arc::clone(&results);
                let snapshot = Arc::clone(&snapshot);
                handles.push(thread::spawn(move || {
                    snapshot.update(i, Some(i + 1));
                    let mut results = results.lock().unwrap();
                    results.push(snapshot.scan(i));
                }));
            }

            for handle in handles {
                handle.join().unwrap();
            }
            
            let views = &*results.lock().unwrap();
            common::assert_maximal_view_exists(views);
            common::assert_views_are_comparable(views);
        });
    }
}

mod bounded_atomic_snapshot {
    use super::*;
    use todc::snapshot::aad_plus_93::BoundedAtomicSnapshot;

    #[test]
    fn test_one_shot_correctness() {
        loom::model(|| {
            let results = Arc::new(Mutex::new(vec![]));
            let mut handles = vec![];
            let snapshot: Arc<BoundedAtomicSnapshot<Option<usize>, NUM_THREADS>> = Arc::new(BoundedAtomicSnapshot::new());

            for i in 0..NUM_THREADS {
                let results = Arc::clone(&results);
                let snapshot = Arc::clone(&snapshot);
                handles.push(thread::spawn(move || {
                    snapshot.update(i, Some(i + 1));
                    let mut results = results.lock().unwrap();
                    results.push(snapshot.scan(i));
                }));
            }

            for handle in handles {
                handle.join().unwrap();
            }
            
            let views = &*results.lock().unwrap();
            common::assert_maximal_view_exists(views);
            common::assert_views_are_comparable(views);
        });
    }
}
