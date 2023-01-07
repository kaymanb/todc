use std::sync::Arc;
use loom::{thread, sync::Mutex};
use todc::snapshot::{Snapshot, aad_plus_93::UnboundedAtomicSnapshot};

const NUM_THREADS: usize = 3;

#[test]
fn test_something() {
    loom::model(|| {
        let results = Arc::new(Mutex::new(vec![]));
        let mut handles = vec![];
        let snapshot: Arc<UnboundedAtomicSnapshot<usize, NUM_THREADS>> = Arc::new(UnboundedAtomicSnapshot::new());

        for i in 0..NUM_THREADS {
            let results = Arc::clone(&results);
            let snapshot = Arc::clone(&snapshot);
            handles.push(thread::spawn(move || {
                snapshot.update(i, i + 1);
                let mut results = results.lock().unwrap();
                results.push(snapshot.scan(i));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
        
        for result in &*results.lock().unwrap() {
            println!("{:?}", result);
        }
        // TODO: Assert that results satisfy snapshot conditions
        assert_eq!(0, 1)
    });
}
