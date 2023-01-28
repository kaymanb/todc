//! A history.
use std::ops::{Index, IndexMut};

#[derive(PartialEq, Eq, Clone, Debug)]
struct Entry<T> {
    id: usize,
    operation: T,
    rtrn: Option<usize>,
    index: Option<usize>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
struct History<T> {
    pub entries: Vec<Entry<T>>,
}

impl<T> History<T> {
    pub fn from_vec(ops: Vec<T>) -> Self {
        let entries = ops.into_iter().enumerate().map(|(i, op)| Entry {
            id: i,
            operation: op,
            rtrn: None,
            index: None,
        });
        Self {
            entries: entries.collect(),
        }
    }

    fn insert(&mut self, mut entry: Entry<T>) {
        let index = entry.index.unwrap();
        entry.index = None;
        self.entries.insert(index, entry);
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
        let return_index = self
            .iter()
            .position(|e| e.id == call_entry.rtrn.unwrap())
            .unwrap();
        let return_entry = self.remove(return_index);
        (call_entry, return_entry)
    }

    fn remove(&mut self, i: usize) -> Entry<T> {
        let mut entry = self.entries.remove(i);
        entry.index = Some(i);
        entry
    }

    fn unlift(&mut self, call: Entry<T>, rtrn: Entry<T>) {
        self.insert(rtrn);
        self.insert(call);
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

    mod from_vec {
        use super::*;

        #[test]
        fn creates_sequential_ids() {
            let history = History::from_vec(vec!["a", "b", "c"]);
            for (i, entry) in history.iter().enumerate() {
                assert_eq!(entry.id, i);
            }
        }
    }

    mod insert {
        use super::*;

        #[test]
        fn is_inverse_of_remove() {
            let mut history = History::from_vec(vec!["a", "b", "c"]);
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
            let mut history = History::from_vec(vec!["a", "b", "c", "d", "e"]);
            history[1].rtrn = Some(3);
            history.lift(1);
            for (entry, letter) in zip(history.iter(), ["a", "c", "e"]) {
                assert_eq!(entry.operation, letter);
            }
        }
    }

    mod remove {
        use super::*;

        #[test]
        fn sets_index_correctly() {
            let mut history = History::from_vec(vec!["a", "b", "c"]);
            let entry = history.remove(1);
            assert_eq!(entry.index.unwrap(), 1);
        }

        #[test]
        fn removes_only_requested_entry() {
            let mut history = History::from_vec(vec!["a", "b", "c", "d"]);
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
            let mut history = History::from_vec(vec!["a", "b", "c", "d", "e"]);
            history[1].rtrn = Some(3);
            let copy = history.clone();

            let (call, rtrn) = history.lift(1);
            history.unlift(call, rtrn);
            assert_eq!(history, copy)
        }
    }
}
