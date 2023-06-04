# todc

`todc` is a library of distributed computing algorithms, written in Rust.

## Overview

This is very experimental. The goal of this library is to bridge the gap between theory and practice by providing _usable_, _understandable_, and _correct_ implementations of algorithms from classic papers. 

### Message Passing

For message passing systems, `todc-net` provides implementations for services that communicate over HTTP. 

### Shared Memory

For shared memory systems, `todc-mem` provides implementations for processes running on a single peice of hardware. 
