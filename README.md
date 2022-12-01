# Theory of Distributed Computing

A library of objects and algorithms in Rust. 

### TODO

- Research and notes on Sequential Consistency vs Linearizability
    - Attiya and Welch paper is best resource for a formal treatment.
- Some sort of implementation test
- Shared snapshot
- `StringRegister`
- `Register<T>` 

## Notes

### Linearizability versus Sequential Consistency

The highest level of synchronization provided to atomics is `Ordering::SeqCst`. 
This is _not_ the same as linearizability. For a formal treatment, see _Sequential Consistency versus Linearizability_ [Attiya and Welch 1994](https://dl.acm.org/doi/pdf/10.1145/176575.176576). 

TODO: Provide an example of how these differ. 
