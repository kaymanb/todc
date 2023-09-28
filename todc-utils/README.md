# todc-utils

[![crates.io](https://img.shields.io/crates/v/todc-utils)](https://crates.io/crates/todc-utils)
[![docs.rs](https://img.shields.io/docsrs/todc-utils)](https://docs.rs/todc-utils/0.1.0/todc_utils/)

Utilities for building and testing distributed algorithms.

## Examples

Consider the following sequential specification for a register containing 
`u32` values.

```rs
use todc_utils::specifications::Specification;

#[derive(Copy, Clone, Debug)]
enum RegisterOp {
    Read(Option<u32>),
    Write(u32),
}

use RegisterOp::{Read, Write};

struct RegisterSpec;

impl Specification for RegisterSpec {
    type State = u32;
    type Operation = RegisterOp;
    
    fn init() -> Self::State {
        0
    }

    fn apply(operation: &Self::Operation, state: &Self::State) -> (bool, Self::State) {
        match operation {
            // A read is valid if the value returned is equal to the
            // current state. Reads always leave the state unchanged.
            Read(value) => match value {
                Some(value) => (value == state, *state),
                None => (false, *state)
            },
            // Writes are always valid, and update the state to be
            // equal to the value being written.
            Write(value) => (true, *value),
        }
    }
}
```    

Using the `Action::Call` and `Action::Response` types, we can model read
and write operations as follows:

* The call of a read operation is modeled by `Call(Read(None))` and a
  response containing the value `x` is modeled by `Response(Read(Some(x)))`.
  We are use an `Option` because the value being read cannot be known until 
  the register responds.
* Similarily, the call of a write operation with the value `y` is modeled
  by `Call(Write(y))` and the response is modeled by `Response(Write(y))`.

Next, we can define a linearizability for this specification, and check some
histories.

```rs
use todc_utils::linearizability::{WGLChecker, history::{History, Action::{Call, Response}}};

type RegisterChecker = WGLChecker<RegisterSpec>;

// A history of sequantial operations is always linearizable.
// PO |------|          Write(0)
// P1          |------| Read(Some(0))
let history = History::from_actions(vec![
    (0, Call(Write(0))),
    (0, Response(Write(0))),
    (1, Call(Read(None))),
    (1, Response(Read(Some(0)))),
]);
assert!(RegisterChecker::is_linearizable(history));

// Concurrent operations might not be linearized
// in the order in which they are called.
// PO |---------------| Write(0)
// P1  |--------------| Write(1)
// P2    |---|          Read(Some(1))
// P3           |---|   Read(Some(0))
let history = History::from_actions(vec![
    (0, Call(Write(0))),
    (1, Call(Write(1))),
    (2, Call(Read(None))),
    (2, Response(Read(Some(1)))),
    (3, Call(Read(None))),
    (3, Response(Read(Some(0)))),
    (0, Response(Write(0))),
    (1, Response(Write(1))),
]);
assert!(RegisterChecker::is_linearizable(history));

// A sequentially consistent history is **not**
// necessarily linearizable.
// PO |---|             Write(0)
// P1 |---|             Write(1)
// P2       |---|       Read(Some(1))
// P3             |---| Read(Some(0))
let history = History::from_actions(vec![
    (0, Call(Write(0))),
    (1, Call(Write(1))),
    (0, Response(Write(0))),
    (1, Response(Write(1))),
    (2, Call(Read(None))),
    (2, Response(Read(Some(1)))),
    (3, Call(Read(None))),
    (3, Response(Read(Some(0)))),
]);
assert!(!RegisterChecker::is_linearizable(history));
```

For examples of using `WGLChecker` to check the linearizability of more
complex histories, see
[`todc-mem/tests/snapshot/common.rs`](https://github.com/kaymanb/todc/blob/main/todc-mem/tests/snapshot/common.rs)
or
[`todc-utils/tests/linearizability/etcd.rs`](https://github.com/kaymanb/todc/blob/main/todc-utils/tests/linearizability/etcd.rs).

