//! `Vec<T>` lenses
use crate::data::{Collection, Data, IndexAddress};
use crate::lens::{Lens, LensCompose, LensIndexExt};
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;

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

#[derive(Clone)]
pub struct VecAddress<T: Data>(usize, Option<T::Address>);

// #26925 impl (when is this going to be fixed?)
impl<T: Data> fmt::Debug for VecAddress<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]", self.0)?;
        if let Some(addr) = &self.1 {
            write!(f, ".{:?}", addr)?;
        }
        Ok(())
    }
}

impl<T: Data> PartialEq for VecAddress<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl<T: Data> Eq for VecAddress<T> {}

impl<T: Data> IndexAddress for VecAddress<T> {
    type Element = T;
    type Index = usize;

    fn index(&self) -> Self::Index {
        self.0
    }

    fn rest(&self) -> &Option<T::Address> {
        &self.1
    }
}

impl<T: Data> Data for Vec<T> {
    type Address = VecAddress<T>;
}

impl<T: Data> Collection for Vec<T> {
    type Index = usize;
    type Element = T;
    type ElementLens = VecLens<T>;

    fn at_index(index: usize) -> VecLens<T> {
        VecLens {
            index,
            _phantom: PhantomData,
        }
    }

    fn get_at(&self, index: Self::Index) -> Option<&Self::Element> {
        self.get(index)
    }

    fn box_iter<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Self::Element> + 'a> {
        Box::new(self.iter())
    }
}

impl<T: Data> Lens for VecLens<T> {
    type Root = Vec<T>;
    type Leaf = T;

    fn address(&self) -> Option<VecAddress<T>> {
        Some(VecAddress(self.index, None))
    }

    fn concat<L, U>(&self, rhs: L) -> Option<VecAddress<T>>
    where
        L: Lens<Root = T, Leaf = U>,
        U: Data,
    {
        Some(VecAddress(self.index, rhs.address()))
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

    fn unprefix(&self, addr: VecAddress<T>) -> Option<Option<T::Address>> {
        if self.index == addr.0 {
            Some(addr.1)
        } else {
            None
        }
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

/*
/// Lookup by sorted key (linear search) in a sorted vector.
#[derive(Copy,Clone,Debug)]
pub struct LinearSearchVecLookupLens<T: Entity>(T::Key);

impl<T: Entity> Lens for LinearSearchVecLookupLens<T> {
    type Root = Vec<T>;
    type Leaf = T;

    fn address(&self) -> _ {
        unimplemented!()
    }

    fn concat<L, U>(&self, rhs: L) -> Vec where L: Lens<Root=Self::Leaf, Leaf=U>, U: Data {
        unimplemented!()
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
*/
