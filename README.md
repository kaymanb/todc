# Theory of Distributed Computing

A library of algorithms used in theory of distributed computing, written in Rust. 

## Integration Tests

Run with the help of [loom](https://github.com/tokio-rs/loom):
```
LOOM_MAX_PREEMPTIONS=3 RUSTFLAGS="--cfg loom" cargo test --test main --release snapshot
```
