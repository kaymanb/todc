//! A sequence of operations applied to a shared object.
use std::collections::VecDeque;
use std::iter::repeat_with;
use std::ops::{Index, IndexMut};

/// A identifier for an [`Entry`]
pub type EntryId = usize;

/// A process identifier.
pub type ProcessId = usize;

/// An action that occurs as part of an operation on a shared object.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Action<T> {
    /// A `Call` indicates the beginning of an operation.
    Call(T),
    /// A `Response` indicates the end of an operation.
    Response(T),
}

/// An entry in a history that represents the call to an operation.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct CallEntry<T> {
    /// The identifier for this [`CallEntry`].
    pub id: EntryId,
    /// The operation being called.
    pub operation: T,
    /// The identifier of the [`ResponseEntry`] that stores the response to this
    /// operation.
    pub response: EntryId,
}

/// An entry in a history that represents the response from an operation.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ResponseEntry<T> {
    /// The identifier for this [`ResponseEntry`].
    pub id: EntryId,
    /// The operation being responded to.
    pub operation: T,
}

/// An entry in a history.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Entry<T> {
    Call(CallEntry<T>),
    Response(ResponseEntry<T>),
}

impl<T> Entry<T> {
    /// Returns a unique identifier for this [`Entry`].
    pub fn id(&self) -> EntryId {
        match self {
            Entry::Call(entry) => entry.id,
            Entry::Response(entry) => entry.id,
        }
    }
}

/// A sequence of operations applied to a shared object.
///
/// A history is a sequence of operations that have been applied to a shared
/// object. Each [`Entry`] in the history indicates either a call to or response
/// from an operation performed by a specific process. It is possible for
/// operations from different processes to be performed concurrently, which
/// is modeled in a history by interleaving the call and response entries
/// for those operations.
///
/// # Examples
///
/// Consider a history of operations performed on a shared register, where
/// processes can write values and read values that have been written. In
/// the following example process `P0` performs a write, after which process
/// `P1` performs a read.
///
/// ```
/// use std::matches;
/// use todc_utils::{History, Action::{Call, Response}};
/// use todc_utils::linearizability::history::Entry;
/// use todc_utils::specifications::register::RegisterOperation::{Read, Write};
///
/// // P0 |--------|            Write("Hello, World!")
/// // P1            |--------| Read("Hello, World!")
/// let actions = vec![
///     (0, Call(Write("Hello, World!"))),
///     (0, Response(Write("World, World!"))),
///     (1, Call(Read(None))),
///     (1, Response(Read(Some("Hello, World!")))),
/// ];
///
/// let history = History::from_actions(actions);
/// assert!(matches!(&history[0], Entry::Call(x)));
/// ```
///
/// In the next example processes `P0`, `P1`, and `P2` each perform
/// a write while `P3` performs three reads. Notice that the reads performed by
/// `P3` occur concurrently with the writes performed by other processes.
///
/// ```
/// # use std::matches;
/// # use todc_utils::{History, Action::{Call, Response}};
/// # use todc_utils::linearizability::history::Entry;
/// # use todc_utils::specifications::register::RegisterOperation::{Read, Write};
/// // P0 |--------------------| Write(0)
/// // P1 |--------------------| Write(1)
/// // P2 |--------------------| Write(2)
/// // P3   |--|                 Read(2)
/// // P3          |--|          Read(1)
/// // P3                 |--|   Read(0)
/// let actions = vec![
///     (0, Call(Write(0))),
///     (1, Call(Write(1))),
///     (2, Call(Write(2))),
///     (3, Call(Read(None))),
///     (3, Response(Read(Some(2)))),
///     (3, Call(Read(None))),
///     (3, Response(Read(Some(1)))),
///     (3, Call(Read(None))),
///     (3, Response(Read(Some(0)))),
///     (0, Response(Write(0))),
///     (1, Response(Write(1))),
///     (2, Response(Write(2))),
/// ];
///
/// let history = History::from_actions(actions);
/// assert!(matches!(&history[0], Entry::Call(x)));
/// ```
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct History<T> {
    pub(super) entries: Vec<Entry<T>>,
    // When an entry is removed from this history, its index is recorded here.
    removed_from: Vec<Option<EntryId>>,
}

impl<T> History<T> {
    /// Creates a history from a sequence of actions.
    ///
    /// # Panics
    ///
    /// Panics if `actions` is empty.
    ///
    /// Panics if the resulting history would be incomplete. That is, if there is some
    /// `Call` action that does not have a corresponding `Response`.
    ///
    /// ```should_panic
    /// # use std::matches;
    /// # use todc_utils::{History, Action::{Call, Response}};
    /// # use todc_utils::specifications::register::RegisterOperation::{Write};
    /// let incomplete_actions = vec![
    ///     (0, Call(Write("Hello"))),
    ///     (1, Call(Write("World"))),
    ///     (0, Response(Write("Hello"))),
    ///     // <-- Missing response to the call by process 1!
    /// ];
    ///
    /// let history = History::from_actions(incomplete_actions);
    /// ```
    pub fn from_actions(actions: Vec<(ProcessId, Action<T>)>) -> Self {
        let (processes, actions): (Vec<ProcessId>, Vec<Action<T>>) = actions.into_iter().unzip();

        let num_processes = processes.iter().max().unwrap();
        let mut calls: Vec<VecDeque<usize>> = repeat_with(VecDeque::new)
            .take(*num_processes + 1)
            .collect();
        let mut responses = calls.clone();
        for (i, process) in processes.iter().enumerate() {
            match &actions[i] {
                Action::Call(_) => calls[*process].push_back(i),
                Action::Response(_) => responses[*process].push_back(i),
            }
        }

        Self {
            entries: actions
                .into_iter()
                .enumerate()
                .map(|(i, action)| match action {
                    Action::Call(operation) => Entry::Call(CallEntry {
                        id: i,
                        operation,
                        response: responses[processes[i]].pop_front().unwrap(),
                    }),
                    Action::Response(operation) => {
                        Entry::Response(ResponseEntry { id: i, operation })
                    }
                })
                .collect(),
            removed_from: repeat_with(|| None).take(processes.len()).collect(),
        }
    }

    // TODO: This operation is very expensive. Implementing History as a doubly-linked list could
    // greatly improve performance.
    pub(super) fn index_of_id(&self, id: EntryId) -> usize {
        self.iter().position(|e| e.id() == id).unwrap()
    }

    /// # Panics
    ///
    /// Panics if input entry was not previously removed from the history.
    fn insert(&mut self, entry: Entry<T>) -> usize {
        match self.removed_from[entry.id()].take() {
            Some(index) => {
                self.entries.insert(index, entry);
                index
            }
            None => panic!(
                "Index that entry {} was removed from is unknown",
                entry.id()
            ),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &Entry<T>> {
        self.entries.iter()
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn lift(&mut self, i: usize) -> (Entry<T>, Entry<T>) {
        match self.remove(i) {
            Entry::Response(_) => panic!("Cannot lift a response entry out of the history"),
            Entry::Call(call) => {
                let response = self.remove(self.index_of_id(call.response));
                (Entry::Call(call), response)
            }
        }
    }

    fn remove(&mut self, i: usize) -> Entry<T> {
        let entry = self.entries.remove(i);
        self.removed_from[entry.id()] = Some(i);
        entry
    }

    pub(super) fn unlift(&mut self, call: Entry<T>, response: Entry<T>) -> (usize, usize) {
        let response_index = self.insert(response);
        let call_index = self.insert(call);
        (call_index, response_index)
    }
}

impl<T> Index<usize> for History<T> {
    type Output = Entry<T>;

    fn index(&self, i: usize) -> &Self::Output {
        self.entries.index(i)
    }
}

impl<T> IndexMut<usize> for History<T> {
    fn index_mut(&mut self, i: usize) -> &mut Self::Output {
        self.entries.index_mut(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::zip;
    use Action::{Call, Response};

    mod from_actions {
        use super::*;

        #[test]
        fn creates_sequential_ids() {
            let history = History::from_actions(vec![
                (0, Call("a")),
                (0, Response("a")),
                (0, Call("b")),
                (0, Response("b")),
            ]);
            for (i, entry) in history.iter().enumerate() {
                assert_eq!(entry.id(), i);
            }
        }

        #[test]
        fn links_actions_of_multiple_processes() {
            let history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (2, Call("c")),
                (0, Response("a")),
                (1, Response("b")),
                (2, Response("c")),
                (0, Call("e")),
                (1, Call("f")),
                (2, Call("g")),
                (0, Response("e")),
                (1, Response("f")),
                (2, Response("g")),
            ]);
            for entry in history.iter() {
                println!("{:?}", entry);
                if let Entry::Call(call) = entry {
                    match &history[history.index_of_id(call.response)] {
                        Entry::Response(response) => assert_eq!(call.operation, response.operation),
                        Entry::Call(_) => panic!("Call entry was linked to another call entry"),
                    }
                }
            }
        }

        #[test]
        fn links_actions_of_single_process() {
            let history = History::from_actions(vec![
                (0, Call("a")),
                (0, Response("a")),
                (0, Call("b")),
                (0, Response("b")),
                (0, Call("c")),
                (0, Response("c")),
            ]);
            for entry in history.iter() {
                if let Entry::Call(call) = entry {
                    match &history[history.index_of_id(call.response)] {
                        Entry::Response(response) => assert_eq!(call.operation, response.operation),
                        Entry::Call(_) => panic!("Call entry was linked to another call entry"),
                    }
                }
            }
        }
    }

    mod insert {
        use super::*;

        #[test]
        fn is_inverse_of_remove() {
            let history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (2, Call("c")),
                (0, Response("a")),
                (1, Response("b")),
                (2, Response("c")),
            ]);
            let mut copy = history.clone();
            let entry = copy.remove(1);
            copy.insert(entry);
            for (copy, entry) in zip(copy.iter(), history.iter()) {
                assert_eq!(copy, entry);
            }
        }
    }

    mod lift {
        use super::*;

        #[test]
        fn removes_call_and_response_entries() {
            let mut history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (2, Call("c")),
                (0, Response("a")),
                (1, Response("b")),
                (2, Response("c")),
            ]);
            history.lift(0);
            for (entry, letter) in zip(history.iter(), ["b", "c", "b", "c"]) {
                match entry {
                    Entry::Call(call) => assert_eq!(call.operation, letter),
                    Entry::Response(resp) => assert_eq!(resp.operation, letter),
                }
            }
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn removes_only_requested_entry() {
            let mut history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (0, Response("a")),
                (1, Response("b")),
            ]);
            match history.remove(1) {
                Entry::Call(call) => assert_eq!(call.operation, "b"),
                Entry::Response(_) => panic!("Removed incorrect entry"),
            }
            for (entry, letter) in zip(history.iter(), ["a", "a", "b"]) {
                match entry {
                    Entry::Call(entry) => assert_eq!(entry.operation, letter),
                    Entry::Response(entry) => assert_eq!(entry.operation, letter),
                }
            }
        }
    }

    mod unlift {
        use super::*;

        #[test]
        fn is_inverse_of_lift() {
            let mut history = History::from_actions(vec![
                (0, Call("a")),
                (1, Call("b")),
                (0, Response("a")),
                (1, Response("b")),
            ]);
            let copy = history.clone();
            let (call, response) = history.lift(0);
            history.unlift(call, response);
            assert_eq!(history, copy)
        }
    }
}
