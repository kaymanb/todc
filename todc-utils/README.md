# todc-utils

Utilities for building and testing distributed algorithms.

## Examples

Determine if a history of operations performed on some shared-object, like 
[`etcd`](https://etcd.io/), is actually linearizable. See `todc-utils/tests/etcd.rs`
for more details.

```rs
use todc_utils::linearizability::WGLChecker;
use todc_utils::specifications::etcd::{history_from_log, EtcdSpecification};

// Define a linearizability checker for an etcd (compare-and-swap) object.
type EtcdChecker = WGLChecker<EtcdSpecification>;

// Create a history of operations based on log output.
let history = history_from_log("todc-utils/tests/linearizability/etcd/etcd_001.log")

// Assert that the history of operations is actually linearizable.
assert!(EtcdChecker::is_linearizable(history));
```
