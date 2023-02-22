use core::time::Duration;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use todc::linearizability::WLGChecker;
use utils::linearizability::specs::etcd::{history_from_log, EtcdSpecification};

const FILE: &str = "benches/linearizability_wlg_checker/etcd_log_005.log";

fn criterion_benchmark(c: &mut Criterion) {
    let history = history_from_log(FILE.to_owned());
    c.bench_function("Check etcd linearizability", |b| {
        b.iter_batched(
            || history.clone(),
            |history| WLGChecker::is_linearizable(EtcdSpecification, history),
            BatchSize::SmallInput,
        )
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(6));
    targets = criterion_benchmark
}
criterion_main!(benches);
