# todc

`todc` is a library of distributed computing algorithms, written in Rust.

## Overview

This is very experimental. The goal of this library is to bridge the gap between 
theory and practice by providing _usable_, _understandable_, and _correct_ 
implementations of algorithms from classic papers. 

### Message Passing

For message passing systems, `todc-net` provides implementations for services 
that communicate over HTTP. 

### Shared Memory

For shared memory systems, `todc-mem` provides implementations for processes 
running on a single peice of hardware. 


## Development

### Code Coverage

Code coverage can be calculated with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov). 
Some tests can only be run when certain features are enabled. To most-accurately
calculate code coverage, run:

```
cargo llvm-cov clean --workspace
cargo llvm-cov --no-report --workspace
cargo llvm-cov --no-report -p todc-mem --features shuttle --test snapshot
cargo llvm-cov --no-report -p todc-net --features turmoil --test abd_95
cargo llvm-cov report --open
```

