use std::{cell::RefCell, rc::Rc};

// TODO: This would likely be much faster if carefully implemeted with `unsafe`
// instead of runtime borrow checking.
pub type EntryLink<T> = Option<Rc<RefCell<Entry<T>>>>;

/// An entry.
///
/// An objects history is represented by a doubly-linked-list of entries, each of which
/// represents either a call or return of an operation performed on the object.
///
/// Entries in a history can either be a _call_ entry, indicating that at operation has started,
/// or a _return_ entry, indicating that an operation has finished. In a _complete_ history, each
/// call entry has a corresponding
pub struct Entry<T> {
    pub id: usize,
    // It can be helpful to introduce entries into a history that do *not*
    // correspond to a particular operations, hence the Option type.
    pub operation: Option<T>,
    pub next: EntryLink<T>,
    pub prev: EntryLink<T>,
    pub rtrn: EntryLink<T>,
}

impl<T> Entry<T> {
    /// Creates a new, blank, entry with the given id.
    pub fn new(id: usize) -> Self {
        Self {
            id,
            operation: None,
            prev: None,
            next: None,
            rtrn: None,
        }
    }

    /// Returns if the entry is the last in its history.
    pub fn is_last(&self) -> bool {
        self.next.is_none()
    }

    /// Returns whether or not the entry is for the call to an operation (versus
    /// the return from an operation.)
    pub fn is_call(&self) -> bool {
        self.rtrn.is_some()
    }

    /// Returns the length of the history beginning at this entry.
    pub fn len(&self) -> usize {
        match &self.next {
            Some(entry) => 1 + entry.borrow().len(),
            None => 1,
        }
    }

    /// Removes the entry from its history.
    ///
    /// The lifted entry retains its links to other entries in the history, which
    /// can be used to re-introduce it into the history later. See [`unlift`].
    ///
    /// TODO: Diagram.
    pub fn lift(&self) {
        // Bypass the entry by linking its predecessor to its successor.
        if let Some(entry) = &self.prev {
            entry.borrow_mut().next = self.next.clone();
        }
        if let Some(entry) = &self.next {
            entry.borrow_mut().prev = self.prev.clone();
        }

        // If it is a call entry, bypass the corresponding return entry.
        if let Some(return_entry) = &self.rtrn {
            if let Some(entry) = &return_entry.borrow().prev {
                entry.borrow_mut().next = return_entry.borrow().next.clone();
            }
            if let Some(entry) = &return_entry.borrow().next {
                entry.borrow_mut().prev = return_entry.borrow().prev.clone();
            }
        }
    }

    /// Re-introduces the entry back into its history.
    ///
    /// See [`lift`].
    ///
    /// TODO: Diagram.
    pub fn unlift(&mut self) {
        // TODO: Implement.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn new_history_of_len(n: usize) -> Rc<RefCell<Entry<usize>>> {
        let mut entry = Rc::new(RefCell::new(Entry::new(n)));
        for i in 1..n {
            let prev = Rc::new(RefCell::new(Entry::<usize>::new(n - i)));
            prev.borrow_mut().next = Some(entry.clone());
            entry.borrow_mut().prev = Some(prev.clone());
            entry = prev;
        }
        entry
    }

    mod test_len {
        use super::*;

        #[test]
        fn test_new_entry_has_length_one() {
            assert_eq!(Entry::<usize>::new(1).len(), 1);
        }

        #[test]
        fn test_history_has_correct_length() {
            let expected = 5;
            let entry = new_history_of_len(expected);
            assert_eq!(entry.borrow().len(), expected);
        }
        
    }

    mod test_lift {
        use super::*;

        #[test]
        fn test_bypasses_entry() {
            let mut entry = new_history_of_len(3);
            let middle = entry.borrow().next.clone().unwrap();
            middle.borrow().lift();
            // One of the entries has been lifted from the history.
            assert_eq!(entry.borrow().len(), 2);
            
            assert_eq!(entry.borrow().id, 1);
            let next = entry.borrow().next.clone().unwrap();
            entry = next;
            // The middle entry with id=2 has been lifted. 
            assert_eq!(entry.borrow().id, 3);
        }

        #[test]
        fn test_entry_retains_links() {
            let start = new_history_of_len(3);
            let middle = start.borrow().next.clone().unwrap();
            middle.borrow().lift();
            assert_eq!(middle.borrow().len(), 2);

            let next = middle.borrow().next.clone();
            assert_eq!(next.unwrap().borrow().id, 3)
        }


    }
}
