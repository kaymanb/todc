# Theory of Distributed Computing

A library of algorithms used in theory of distributed computing, written in Rust. 

TODO: Refactor README for workspace

## Tests

```
cargo test
```

Coverage report available with `cargo llvm-cov --html --all`.

### Testing shared-memory

Run with the help of [loom](https://github.com/tokio-rs/loom):
```
LOOM_MAX_PREEMPTIONS=3 cargo test --test main --release --features loom
```

## Benchmarks

Run with the help of [criterion](https://github.com/bheisler/criterion.rs)
```
cargo criterion
```
