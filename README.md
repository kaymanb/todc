# Theory of Distributed Computing

A library of algorithms used in theory of distributed computing, written in Rust. 

## Tests

```
cargo test
```

Coverage report available with `cargo llvm-cov --html`.

### Testing shared-memory

Run with the help of [loom](https://github.com/tokio-rs/loom):
```
LOOM_MAX_PREEMPTIONS=3 RUSTFLAGS="--cfg loom" cargo test --test main --release snapshot
```

## Benchmarks

Run with the help of [criterion](https://github.com/bheisler/criterion.rs)
```
cargo criterion
```
