use std::sync::Arc;

use loom::{sync::Mutex, thread};
use todc::snapshot::Snapshot;

use super::common;
use super::common::HistoriedSnapshot;

const NUM_THREADS: usize = 3;

mod unbounded_atomic_snapshot {
    use super::*;
    use todc::snapshot::aad_plus_93::UnboundedAtomicSnapshot;

    #[test]
    fn test_one_shot_correctness() {
        loom::model(|| {
            println!("Loom Model");
            let results = Arc::new(Mutex::new(vec![]));
            let mut handles = vec![];
            let snapshot: Arc<
                HistoriedSnapshot<NUM_THREADS, UnboundedAtomicSnapshot<Option<usize>, NUM_THREADS>>,
            > = Arc::new(HistoriedSnapshot::new());

            for i in 0..NUM_THREADS {
                let results = Arc::clone(&results);
                let snapshot = Arc::clone(&snapshot);
                handles.push(thread::spawn(move || {
                    let update = snapshot.update(i, Some(i + 1));
                    let scan = snapshot.scan(i);
                    let mut results = results.lock().unwrap();
                    results.push(update);
                    results.push(scan);
                }));
            }

            for handle in handles {
                handle.join().unwrap();
            }

            let mut results = &mut *results.lock().unwrap();
            results.sort_by(|a, b| a.start.cmp(&b.start));
            for event in results {
                println!("{:?}", event);
            }
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
            let snapshot: Arc<BoundedAtomicSnapshot<Option<usize>, NUM_THREADS>> =
                Arc::new(BoundedAtomicSnapshot::new());

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
