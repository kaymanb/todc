//! A history.

struct Entry<T> {
    id: usize,
    operation: T,
    rtrn: Option<usize>,
    idx: Option<usize>, // TODO: Rename to index
}

struct History<T> {
    pub entries: Vec<Entry<T>>,
}

impl<T> History<T> {
    pub fn from_vec(ops: Vec<T>) -> Self {
        let len = ops.len();
        let entries = ops.into_iter().enumerate().map(|(i, op)| {
            Entry {
                id: i,
                operation: op,
                rtrn: None,
                idx: None
            }
        });
        Self {
            entries: entries.collect(),
        }
    }

    fn insert(&mut self, mut entry: Entry<T>) {
        let idx = entry.idx.unwrap(); 
        entry.idx = None;
        self.entries.insert(idx, entry);
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
        let return_index = self.iter().position(|e| e.id == call_entry.rtrn.unwrap()).unwrap();
        let return_entry = self.remove(return_index);
        (call_entry, return_entry)
    }

    fn remove(&mut self, i: usize) -> Entry<T> {
        let mut entry = self.entries.remove(i);
        entry.idx = Some(i);
        entry
    }
    
    fn unlift(&mut self, call: Entry<T>, rtrn: Entry<T>) {
        self.insert(rtrn);
        self.insert(call);
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
            history.entries[1].rtrn = Some(3); // TODO: Implement Index and IndexMut for History
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
            assert_eq!(entry.idx.unwrap(), 1);
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
            history.entries[1].rtrn = Some(3);
            let (call, rtrn) = history.lift(1);
            history.unlift(call, rtrn);
            for (entry, letter) in zip(history.iter(), ["a", "b", "c", "d", "e"]) {
                assert_eq!(entry.operation, letter);
            }
        }
    }
}
