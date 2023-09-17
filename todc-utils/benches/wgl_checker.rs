use core::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

use todc_utils::linearizability::WGLChecker;
use todc_utils::specifications::etcd::{history_from_log, EtcdSpecification};

const LOG_FILE: &str = "benches/static/etcd_log_005.log";

// Checks that a relatively complex `etcd` history is in-fact linearizable.
fn criterion_benchmark(c: &mut Criterion) {
    let history = history_from_log(LOG_FILE.to_owned());
    c.bench_function("WGLChecker - check linearizability of etcd log", |b| {
        b.iter_batched(
            || history.clone(),
            WGLChecker::<EtcdSpecification>::is_linearizable,
            BatchSize::SmallInput,
        )
    });
}

criterion_group! {
    name = wlg_checker;
    config = Criterion::default().measurement_time(Duration::from_secs(6));
    targets = criterion_benchmark
}
criterion_main! { wlg_checker }
