use crate::model::{FromValue, Value};
use kyute_common::Atom;
use std::{fmt, fmt::Formatter, marker::PhantomData};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Metadata
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A token that represents a well-known metadata entry.
pub struct Metadata<T> {
    pub name: &'static str,
    _phantom: PhantomData<T>,
}

impl<T: FromValue> Metadata<T> {
    pub const fn new(name: &'static str) -> Metadata<T> {
        Metadata {
            name,
            _phantom: PhantomData,
        }
    }
}

impl<T> Copy for Metadata<T> {}

impl<T> Clone for Metadata<T> {
    fn clone(&self) -> Self {
        Metadata {
            name: self.name,
            _phantom: PhantomData,
        }
    }
}

impl<T> fmt::Debug for Metadata<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Metadata").field(&self.name).finish()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Well-known metadata entries
////////////////////////////////////////////////////////////////////////////////////////////////////

pub const SCHEMA: Metadata<Atom> = Metadata::new("schema");
pub const OPERATOR: Metadata<Atom> = Metadata::new("operator");
