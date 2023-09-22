# todc-mem

Algorithms for shared-memory distributed systems.


## Tests

Some tests make use of [shuttle](https://github.com/awslabs/shuttle) for 
_randomized concurrency testing_. To run tests that require this feature, do:
```
cargo test --features shuttle --test MODULE --release
```
