//! `Vec<T>` lenses
use crate::db::Data;
use std::marker::PhantomData;
use crate::lens::{Lens, Path, LensIndexExt, LensCompose, Entity, LensLookupExt};
use crate::lens::EntityKey;

/// A lens that looks at a particular item in a vector.
///
/// It implements `Lens<Vec<T>,T>`.
#[derive(Clone, Debug)]
pub struct VecLens<T: Data> {
    index: usize,
    _phantom: PhantomData<T>,
}

impl<T: Data> VecLens<T> {
    pub fn new(index: usize) -> VecLens<T> {
        VecLens {
            index,
            _phantom: PhantomData,
        }
    }
}

impl<T: Data> Lens for VecLens<T> {
    type Root = Vec<T>;
    type Leaf = T;

    fn path(&self) -> Path<Vec<T>, T> {
        Path::index(self.index)
    }

    fn get<'a>(&self, data: &'a Vec<T>) -> &'a T {
        &data[self.index]
    }

    fn get_mut<'a>(&self, data: &'a mut Vec<T>) -> &'a mut T {
        &mut data[self.index]
    }

    fn try_get<'a>(&self, data: &'a Self::Root) -> Option<&'a Self::Leaf> {
        data.get(self.index)
    }

    fn try_get_mut<'a>(&self, data: &'a mut Self::Root) -> Option<&'a mut Self::Leaf> {
        data.get_mut(self.index)
    }
}


impl<T, K> LensIndexExt for K
    where
        T: Data,
        K: Lens<Leaf = Vec<T>>,
{
    type Output = LensCompose<K, VecLens<T>>;
    fn index(&self, i: usize) -> Self::Output {
        LensCompose(self.clone(), VecLens::new(i))
    }
}

/// Lookup by sorted key (linear search) in a sorted vector.
#[derive(Copy,Clone,Debug)]
pub struct LinearSearchVecLookupLens<T: Entity>(T::Key);

impl<T: Entity> Lens for LinearSearchVecLookupLens<T> {
    type Root = Vec<T>;
    type Leaf = T;

    fn path(&self) -> Path<Self::Root, Self::Leaf> {
        Path::key(self.0.to_u64())
    }

    fn get<'a>(&self, data: &'a Self::Root) -> &'a Self::Leaf {
        data.iter().find(|&item| item.key() == self.0).unwrap()
    }

    fn get_mut<'a>(&self, data: &'a mut Self::Root) -> &'a mut Self::Leaf {
        data.iter_mut().find(|item| item.key() == self.0).unwrap()
    }

    fn try_get<'a>(&self, data: &'a Self::Root) -> Option<&'a Self::Leaf> {
        data.iter().find(|&item| item.key() == self.0)
    }

    fn try_get_mut<'a>(&self, data: &'a mut Self::Root) -> Option<&'a mut Self::Leaf> {
        data.iter_mut().find(|item| item.key() == self.0)
    }
}

impl<T, L> LensLookupExt for L where
    T: Entity,
    L: Lens<Leaf=Vec<T>>,
{
    type Key = T::Key;
    type Output = LinearSearchVecLookupLens<T>;

    fn by_key(&self, key: Self::Key) -> LinearSearchVecLookupLens<T> {
        LinearSearchVecLookupLens(key)
    }
}
