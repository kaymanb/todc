# todc

[![CI](https://github.com/kaymanb/todc/actions/workflows/ci.yml/badge.svg)](https://github.com/kaymanb/todc/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/kaymanb/todc/graph/badge.svg?token=BP1WOBRO9R)](https://codecov.io/gh/kaymanb/todc)

`todc` is a collection of Rust crates for distributed computing.

## Overview

This is very experimental. The goal of this library is to bridge the gap between 
theory and practice by providing _usable_, _understandable_, and _correct_ 
implementations of algorithms from classic papers. 

### Message Passing

For message passing systems, [`todc-net`](https://github.com/kaymanb/todc/tree/main/todc-net) 
provides implementations for services that communicate over HTTP. 

### Shared Memory

For shared memory systems, [`todc-mem`](https://github.com/kaymanb/todc/tree/main/todc-mem) 
provides implementations for processes running on a single peice of hardware. 

### Utilities

For general utilities, [`todc-utils`](https://github.com/kaymanb/todc/tree/main/todc-utils) 
provides helpful tools for building and testing distributed systems.

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

