use crate::model::{Atom, Metadata, Path, TypeDesc, Value};
use imbl::HashMap;
use kyute::Data;
use lazy_static::lazy_static;
use rusqlite::params;
use std::{borrow::Cow, ops::Deref};

////////////////////////////////////////////////////////////////////////////////////////////////////
// AttributeAny
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A parameter of a `Node`.
#[derive(Clone, Debug)]
pub struct Param {
    pub(crate) rev: u64,

    /// Unique attribute ID.
    pub id: i64,

    /// Path of this object in the document tree. Contains the name of the object.
    pub path: Path,

    /// Type descriptor.
    pub ty: TypeDesc,

    /// Value of the attribute.
    pub value: Option<Value>,

    /// Connection.
    pub connection: Option<Path>,

    /// Metadata
    pub metadata: HashMap<Atom, Value>,
}

impl Param {
    pub(crate) fn new(id: i64, path: Path, ty: TypeDesc, value: Option<Value>, connection: Option<Path>) -> Param {
        Param {
            rev: 0,
            id,
            path,
            ty,
            value,
            connection,
            metadata: Default::default(),
        }
    }

    pub(crate) fn removed(&self) -> bool {
        self.id == 0
    }

    /// Name of the property
    pub fn name(&self) -> Atom {
        self.path.name()
    }

    /// Type ID of the property
    pub fn type_id(&self) -> &TypeDesc {
        &self.ty
    }

    pub fn dump(&self, indent: usize) {
        println!("{:indent$}name  : {}", "", self.name(), indent = indent);
        println!("{:indent$}type  : {:?}", "", self.ty, indent = indent);
        println!("{:indent$}value : {:?}", "", self.value, indent = indent);
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// AttributeType
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait implemented by types that can be the type of a node attribute.
pub trait AttributeType {
    /// Returns the name of the type.
    fn name() -> Atom;
}

macro_rules! impl_attribute_type {
    ($t:ty, $id:ident, $atom:ident) => {
        lazy_static! {
            static ref $atom: Atom = Atom::from(std::stringify!($id));
        }

        impl AttributeType for $t {
            fn name() -> Atom {
                $atom.clone()
            }
        }
    };
}

impl_attribute_type!(f64, f64, ATTRIBUTE_TYPE_F64);
impl_attribute_type!(Atom, atom, ATTRIBUTE_TYPE_ATOM);
impl_attribute_type!(String, string, ATTRIBUTE_TYPE_STRING);
