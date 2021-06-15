use druid::Data;
use std::sync::Arc;
use std::ops::{Index, IndexMut};

/// Ordered collection model.
pub trait OrderedCollection: Data {

    /// The type of this collection's items
    type Item: Data;

    /// Returns the number of items in the collection
    fn item_count(&self) -> usize;

    /// Returns a reference to the item at the given index.
    fn get(&self, index: usize) -> &Self::Item;

    /// Returns a mutable reference to the item at the given index.
    fn get_mut(&mut self, index: usize) -> &mut Self::Item;

    /// Returns a new collection with the given element replaced.
    fn replace(&mut self, old: &Self::Item, new: &Self::Item) -> bool {
        for i in 0..self.item_count() {
            if self.get(i).same(old) {
                *self.get_mut(i) = new.clone();
                // return now, since we assume that all items are unique
                return true;
            }
        }
        false
    }
}

impl<T: Data + 'static> OrderedCollection for Arc<Vec<T>> {
    type Item = T;

    fn item_count(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> &Self::Item {
        self.index(index)
    }

    fn get_mut(&mut self, index: usize) -> &mut Self::Item {
        Arc::make_mut(self).index_mut(index)
    }
}
