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

#### Features
- [`AtomicRegister`](https://docs.rs/todc-net/0.1.0/todc_net/register/abd_95/struct.AtomicRegister.html), a simluation of 
  an atomic shared-memory register, as described by Attiya, Bar-Noy and Dolev [[ABD95]](https://dl.acm.org/doi/pdf/10.1145/200836.200869).

### Shared Memory

For shared memory systems, [`todc-mem`](https://github.com/kaymanb/todc/tree/main/todc-mem) 
provides implementations for processes running on a single peice of hardware. 

#### Features
- [`AtomicRegister`](https://docs.rs/todc-mem/0.1.0/todc_mem/register/struct.AtomicRegister.html), a shared-memory register
  backed by 64 bits of "atomic" memory.
- [`UnboundedSnapshot`](https://docs.rs/todc-mem/0.1.0/todc_mem/snapshot/aad_plus_93/index.html) and 
  [`BoundedSnapshot`](https://docs.rs/todc-mem/0.1.0/todc_mem/snapshot/aad_plus_93/index.html), wait-free
  snapshots that requires $\Theta(n^2)$ operations, as described by Afek et al. [[AAD+93]](https://dl.acm.org/doi/10.1145/153724.153741).
- [`LatticeMutexSnapshot`](https://docs.rs/todc-mem/0.1.0/todc_mem/snapshot/ar_98/index.html), an
  $M$-shot snapshot that requires $O(n \log n)$ operations, as described by Attiya and Rachman [[AR98]](https://epubs.siam.org/doi/10.1137/S0097539795279463).
  

### Utilities

For general utilities, [`todc-utils`](https://github.com/kaymanb/todc/tree/main/todc-utils) 
provides helpful tools for building and testing distributed systems.

#### Features
- [`WGLChecker`](https://docs.rs/todc-utils/0.1.0/todc_utils/linearizability/struct.WGLChecker.html) a fast linearizability
  checker, based on work by Wing and Gong [[WG93]](https://www.cs.cmu.edu/~wing/publications/WingGong93.pdf), 
  Lowe [[L17]](http://www.cs.ox.ac.uk/people/gavin.lowe/LinearizabiltyTesting/), and Horn and Kroenig [[HK15]](https://arxiv.org/abs/1504.00204).

## Development

### Code Coverage

Code coverage can be calculated with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov). 
Some tests can only be run when certain features are enabled. To most-accurately
calculate code coverage, check the [the `coverage` step of CI](https://github.com/kaymanb/todc/blob/main/.github/workflows/ci.yml#L65).
