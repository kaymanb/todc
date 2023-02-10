//! A history.
use std::iter::repeat_with;
use std::ops::{Index, IndexMut};

type EntryID = usize;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Entry<T> {
    pub id: EntryID,
    pub operation: T,
    pub response: Option<EntryID>,
}

impl<T> Entry<T> {
    pub fn is_call(&self) -> bool {
        self.response.is_some()
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct History<T> {
    pub entries: Vec<Entry<T>>,
    // When an entry is removed from this history, its index is recorded here.
    removed_from: Vec<Option<usize>>,
}

impl<T> History<T> {
    pub fn from_ops(ops: Vec<T>) -> Self {
        let num_entries = ops.len();
        let entries = ops.into_iter().enumerate().map(|(i, op)| Entry {
            id: i,
            operation: op,
            response: None,
        });
        Self {
            entries: entries.collect(),
            removed_from: repeat_with(|| None).take(num_entries).collect(),
        }
    }

    /// # Panics
    ///
    /// Panics if `ops` is empty.
    // pub fn from_ops_with_ids(ops: Vec<(usize, T)>) -> Self {
    //     let (ids, ops): (Vec<usize>, Vec<T>) = ops.into_iter().unzip();
    //     let mut history = Self::from_ops(ops);

    //     // TODO: Figure this out... What if this isn't an invocation>?
    //     let num_processes = ids.iter().max().unwrap();
    //     let calls: Vec<Option<usize>> = repeat_with(|| None).take(*num_processes).collect();
    //     for (index, id) in ids.iter().enumerate() {
    //         match calls[id] {
    //             Some(invoke_idx) => {
    //                 history[invoke_idx].response = Some(index)
    //             }
    //         }
    //     }
    // }

    pub fn index_of(&self, id: EntryID) -> usize {
        self.iter().position(|e| e.id == id).unwrap()
    }

    /// # Panics
    ///
    /// Panics if input entry was not previously removed from the history.
    fn insert(&mut self, entry: Entry<T>) -> usize {
        match self.removed_from[entry.id].take() {
            Some(index) => {
                self.entries.insert(index, entry);
                index
            }
            None => panic!("Index that entry {} was removed from is unknown", entry.id),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Entry<T>> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn lift(&mut self, i: usize) -> (Entry<T>, Entry<T>) {
        let call_entry = self.remove(i);
        // Use types to prevent these unwraps.
        let return_index = self.index_of(call_entry.response.unwrap());
        let return_entry = self.remove(return_index);
        (call_entry, return_entry)
    }

    fn remove(&mut self, i: usize) -> Entry<T> {
        let entry = self.entries.remove(i);
        self.removed_from[entry.id] = Some(i);
        entry
    }

    pub fn unlift(&mut self, call: Entry<T>, response: Entry<T>) -> (usize, usize) {
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

    mod from_ops {
        use super::*;

        #[test]
        fn creates_sequential_ids() {
            let history = History::from_ops(vec!["a", "b", "c"]);
            for (i, entry) in history.iter().enumerate() {
                assert_eq!(entry.id, i);
            }
        }
    }

    mod insert {
        use super::*;

        #[test]
        fn is_inverse_of_remove() {
            let mut history = History::from_ops(vec!["a", "b", "c"]);
            let entry = history.remove(1);
            history.insert(entry);
            for (entry, letter) in zip(history.iter(), ["a", "b", "c"]) {
                assert_eq!(entry.operation, letter);
            }
        }
    }

    mod lift {
        use super::*;

        #[test]
        fn removes_call_and_return_entries() {
            let mut history = History::from_ops(vec!["a", "b", "c", "d", "e"]);
            history[1].response = Some(3);
            history.lift(1);
            for (entry, letter) in zip(history.iter(), ["a", "c", "e"]) {
                assert_eq!(entry.operation, letter);
            }
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn removes_only_requested_entry() {
            let mut history = History::from_ops(vec!["a", "b", "c", "d"]);
            assert_eq!("b", history.remove(1).operation);
            for (entry, letter) in zip(history.iter(), ["a", "c", "d"]) {
                assert_eq!(entry.operation, letter);
            }
        }
    }

    mod unlift {
        use super::*;

        #[test]
        fn is_inverse_of_lift() {
            let mut history = History::from_ops(vec!["a", "b", "c", "d", "e"]);
            history[1].response = Some(3);
            let copy = history.clone();

            let (call, response) = history.lift(1);
            history.unlift(call, response);
            assert_eq!(history, copy)
        }
    }
}
