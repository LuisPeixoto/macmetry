//! A fixed-size buffer that forgets the oldest item when it is full.

use std::collections::VecDeque;

/// Keeps the most recent `capacity` items.
#[derive(Debug, Clone)]
pub struct Ring<T> {
    items: VecDeque<T>,
    capacity: usize,
}

impl<T> Ring<T> {
    /// A ring that holds at most `capacity` items. A capacity of 0 is raised to 1.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self { items: VecDeque::with_capacity(capacity.min(4096)), capacity }
    }

    /// Adds an item and returns the one that was pushed out, if any.
    pub fn push(&mut self, item: T) -> Option<T> {
        let evicted = if self.items.len() == self.capacity { self.items.pop_front() } else { None };
        self.items.push_back(item);
        evicted
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn last(&self) -> Option<&T> {
        self.items.back()
    }

    /// Oldest first.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.items.iter()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forgets_the_oldest() {
        let mut ring = Ring::new(3);
        assert_eq!(ring.push(1), None);
        ring.push(2);
        ring.push(3);
        assert_eq!(ring.push(4), Some(1));
        assert_eq!(ring.iter().copied().collect::<Vec<_>>(), vec![2, 3, 4]);
        assert_eq!(ring.last(), Some(&4));
        assert_eq!(ring.len(), 3);
    }

    #[test]
    fn zero_capacity_still_holds_one() {
        let mut ring = Ring::new(0);
        ring.push("a");
        ring.push("b");
        assert_eq!(ring.iter().copied().collect::<Vec<_>>(), vec!["b"]);
        assert_eq!(ring.capacity(), 1);
    }

    #[test]
    fn clear_empties_it() {
        let mut ring = Ring::new(2);
        ring.push(1);
        ring.clear();
        assert!(ring.is_empty());
    }
}
