use std::sync::Arc;
use loom::{thread, sync::Mutex};
use todc::snapshot::Snapshot;

use super::common;

const NUM_THREADS: usize = 3;

mod atomic_snapshot {
    use super::*;
    use todc::snapshot::ar_98::AtomicSnapshot;

    #[test]
    fn test_one_shot_correctness() {
        loom::model(|| {
            let results = Arc::new(Mutex::new(vec![]));
            let mut handles = vec![];
            let snapshot: Arc<AtomicSnapshot<Option<usize>, NUM_THREADS, 8>> = Arc::new(AtomicSnapshot::new());

            for i in 0..NUM_THREADS {
                let results = Arc::clone(&results);
                let snapshot = Arc::clone(&snapshot);
                handles.push(thread::spawn(move || {
                    snapshot.update(i, Some(i + 1));
                    let view = snapshot.scan(i);
                    let mut results = results.lock().unwrap();
                    results.push(view);

                    assert_eq!(Some(i + 1), view[i]);
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
