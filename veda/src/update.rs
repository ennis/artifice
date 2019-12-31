use crate::data::Data;
use crate::{Change, Lens};

/// Represents an edit on a value of type T (i.e. a modification of part of the object).
pub trait Update<T: Data> {
    fn apply(&mut self, model: &mut T) -> Change;
    fn address(&self) -> Option<T::Address>;
}

/// Represents a partial change to an aggregate model.
pub struct Replace<K: Lens> {
    lens: K,
    value: K::Leaf,
}

impl<K: Lens> Replace<K> {
    pub fn new(lens: K, value: K::Leaf) -> Replace<K> {
        Replace { lens, value }
    }
}

impl<U, K> Update<U> for Replace<K>
where
    U: Data,
    K: Lens<Root = U>,
{
    fn apply(&mut self, model: &mut U) -> Change {
        *self.lens.get_mut(model) = self.value.clone();
        Change::replacement()
    }

    fn address(&self) -> Option<U::Address> {
        self.lens.address()
    }
}

/// Append operation.
pub struct Append<A: Data, K: Lens> {
    lens: K,
    element: A,
}

impl<A: Data, K: Lens> Append<A, K> {
    pub fn new(into: K, element: A) -> Append<A, K> {
        Append {
            lens: into,
            element,
        }
    }
}

// Append to Vec<A>
impl<A: Data, K: Lens<Leaf = Vec<A>>> Update<K::Root> for Append<A, K> {
    fn apply(&mut self, model: &mut K::Root) -> Change {
        self.lens.get_mut(model).push(self.element.clone());
        Change::replacement() // TODO more precise description ?
    }

    fn address(&self) -> Option<<K::Root as Data>::Address> {
        self.lens.address()
    }
}

//--------------------------------------------------------------------------------------------------
/// Insert operation
pub struct Insert<A: Data, K: Lens, I: Clone> {
    lens: K,
    index: I,
    element: A,
}

impl<A: Data, K: Lens, I: Clone> Insert<A, K, I> {
    pub fn new(into: K, index: I, element: A) -> Insert<A, K, I> {
        Insert {
            lens: into,
            index,
            element,
        }
    }
}

// Insert into Vec<A>
impl<A: Data, K: Lens<Leaf = Vec<A>>> Update<K::Root> for Insert<A, K, usize> {
    fn apply(&mut self, model: &mut K::Root) -> Change {
        self.lens
            .get_mut(model)
            .insert(self.index, self.element.clone());
        Change::replacement()
    }

    fn address(&self) -> Option<<<K as Lens>::Root as Data>::Address> {
        self.lens.address()
    }
}
