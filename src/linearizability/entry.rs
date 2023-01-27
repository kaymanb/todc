use std::{cell::RefCell, rc::Rc};

// TODO: This would likely be much faster if carefully implemeted with `unsafe`
// instead of runtime borrow checking.
pub type EntryLink<T> = Option<Rc<RefCell<Entry<T>>>>;

/// An entry.
///
/// Entries in a history can either be a _call_ entry, indicating that at operation has started,
/// or a _return_ entry, indicating that an operation has finished. In a _complete_ history, each
/// call entry has a corresponding return entry.
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
    /// Returns a new, blank, entry with the given id.
    pub fn new(id: usize) -> Self {
        Self {
            id,
            operation: None,
            prev: None,
            next: None,
            rtrn: None,
        }
    }

    /// Returns a link to the first in a history of entries, each with their given id.
    ///
    /// # Panics:
    /// Panics if the given iterator of ids is empty.
    ///
    /// # Example:
    /// ```
    /// # use todc::linearizability::Entry;
    /// let first = Entry::<u32>::from_iter(0..3);
    /// assert_eq!(first.borrow().id, 0);
    /// let second = first.borrow().next.clone().unwrap();
    /// assert_eq!(second.borrow().id, 1);
    /// let third = second.borrow().next.clone().unwrap();
    /// assert_eq!(third.borrow().id, 2);
    /// ```
    pub fn from_iter(ids: impl DoubleEndedIterator<Item = usize>) -> Rc<RefCell<Entry<T>>> {
        let mut entry: EntryLink<T> = None;
        for id in ids.rev() {
            let prev = Rc::new(RefCell::new(Entry::<T>::new(id)));
            match entry {
                Some(entry) => {
                    prev.borrow_mut().next = Some(entry.clone());
                    entry.borrow_mut().prev = Some(prev.clone());
                }
                None => prev.borrow_mut().next = None,
            }
            entry = Some(prev);
        }
        entry.unwrap()
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
    /// can be used to re-introduce it into the history later. See
    /// [`unlift`][Entry::unlift].
    ///
    /// # Examples:
    /// ```
    /// # use todc::linearizability::Entry;
    /// let first = Entry::<u32>::from_iter(0..4);
    /// let mut second = first.borrow().next.clone().unwrap();
    /// let third = second.borrow().next.clone();
    /// second.borrow_mut().rtrn = third;
    ///  
    /// // Calling .lift() removes the second entry, and its corresponding
    /// // return entry from the history.
    /// assert_eq!(first.borrow().len(), 4);
    /// second.borrow().lift();
    /// assert_eq!(first.borrow().len(), 2);
    /// ```
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
    /// See [`lift`][Entry::lift].
    ///
    /// TODO: Example.
    pub fn unlift(&self) {
        if let Some(return_entry) = &self.rtrn {
            if let Some(entry) = &return_entry.borrow().prev {
                entry.borrow_mut().next = Some(return_entry.clone());
            }
            if let Some(entry) = &return_entry.borrow().next {
                entry.borrow_mut().prev = Some(return_entry.clone());
            }
        }
        // TODO: Pick up here.
        if let Some(entry) = &self.prev {
            entry.borrow_mut().next = Some(Rc::new(RefCell::new(self)));
        }
        if let Some(entry) = &self.next {
            entry.borrow_mut().prev = self.prev.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod test_len {
        use super::*;

        #[test]
        fn test_new_entry_has_length_one() {
            assert_eq!(Entry::<u32>::new(1).len(), 1);
        }

        #[test]
        fn test_history_has_correct_length() {
            let entry = Entry::<u32>::from_iter(0..5);
            assert_eq!(entry.borrow().len(), 5);
        }
    }

    mod test_lift {
        use super::*;

        #[test]
        fn test_bypasses_entry() {
            let first = Entry::<u32>::from_iter(0..3);
            let second = first.borrow().next.clone().unwrap();
            second.borrow().lift();

            // One of the entries has been lifted from the history.
            assert_eq!(first.borrow().len(), 2);
            let new_second = first.borrow().next.clone().unwrap();
            // The (original) second entry with id=1 has been lifted.
            assert_eq!(first.borrow().id, 0);
            assert_eq!(new_second.borrow().id, 2);
        }

        #[test]
        fn test_bypasses_return_entry() {
            let first = Entry::<u32>::from_iter(0..3);
            let second = first.borrow().next.clone().unwrap();
            let third = second.borrow().next.clone();
            second.borrow_mut().rtrn = third;
            second.borrow().lift();

            // Both call and return entries have been lifted from the history.
            assert_eq!(first.borrow().len(), 1);
        }

        #[test]
        fn test_retains_links_to_sibling_entries() {
            let first = Entry::<u32>::from_iter(0..3);
            let second = first.borrow().next.clone().unwrap();
            second.borrow().lift();

            // prev, and rtrn all continue to point to their respective entries.
            assert_eq!(second.borrow().len(), 2);
            let next = second.borrow().next.clone().unwrap();
            assert_eq!(next.borrow().id, 2);
            let prev = second.borrow().prev.clone().unwrap();
            assert_eq!(prev.borrow().id, 0);
        }

        #[test]
        fn test_retains_links_to_return_entry() {
            let first = Entry::<u32>::from_iter(0..3);
            let second = first.borrow().next.clone().unwrap();
            let third = second.borrow().next.clone();
            second.borrow_mut().rtrn = third;
            second.borrow().lift();

            // Both call and return entries have been lifted from the history.
            let rtrn = second.borrow().rtrn.clone().unwrap();
            assert_eq!(rtrn.borrow().id, 2);
        }
    }
}
