# todc-net

Algorithms for message-passing (HTTP) distributed systems.

## Tests

Some tests make use of [turmoil](https://github.com/tokio-rs/turmoil) to
simulate latency and failures within a network. To run tests that require this
feature, do:
```
cargo test --features turmoil --test MODULE
```
