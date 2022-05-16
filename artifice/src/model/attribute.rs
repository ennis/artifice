use crate::model::{file::DocumentDatabase, Atom, Metadata, Path, Value};
use artifice::model::EditAction;
use imbl::HashMap;
use kyute::Data;
use lazy_static::lazy_static;
use rusqlite::params;
use std::{borrow::Cow, ops::Deref};

////////////////////////////////////////////////////////////////////////////////////////////////////
// AttributeAny
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An attribute of a `Node`.
#[derive(Clone, Debug, Data)]
pub struct AttributeAny {
    pub(crate) rev: u64,

    /// Unique attribute ID.
    pub id: i64,

    /// Path of this object in the document tree. Contains the name of the object.
    pub path: Path,

    /// Type identifier.
    pub ty: Atom,

    /// Value of the attribute.
    pub value: Option<Value>,

    /// Connection.
    pub connection: Option<Path>,

    /// Metadata
    pub metadata: HashMap<Atom, Value>,
}

impl AttributeAny {
    pub(crate) fn new(id: i64, path: Path, ty: Atom, value: Option<Value>, connection: Option<Path>) -> AttributeAny {
        AttributeAny {
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
    pub fn type_id(&self) -> &Atom {
        &self.ty
    }

    pub fn dump(&self, indent: usize) {
        println!("{:indent$}name  : {}", "", self.name(), indent = indent);
        println!("{:indent$}type  : {}", "", self.ty, indent = indent);
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

/*#[derive(Clone, Debug, Data)]
pub struct TypedAttribute<T> {
    pub attribute: Attribute,
    pub value: T,
}*/

////////////////////////////////////////////////////////////////////////////////////////////////////
// EditProxies
////////////////////////////////////////////////////////////////////////////////////////////////////

/*
#[derive(Debug)]
pub(crate) struct MetadataEdit {
    pub(crate) name: Atom,
    pub(crate) action: EditAction,
    pub(crate) value: Option<Value>,
}

#[derive(Debug)]
pub struct AttributeEditProxy<'a> {
    attribute: Cow<'a, AttributeAny>,
    removed: bool,
    init_rev: u64,
    db: &'a mut DocumentDatabase,
}

impl<'a> Deref for AttributeEditProxy<'a> {
    type Target = AttributeAny;
    fn deref(&self) -> &Self::Target {
        self.attribute.deref()
    }
}

pub enum AttributeChange {
    /// The attribute was modified. Contains the new value of the attribute.
    Modified(AttributeAny),
    /// The attribute was removed.
    Removed,
}

impl<'a> AttributeEditProxy<'a> {
    pub(crate) fn borrowed(attribute: &'a AttributeAny, db: &'a mut DocumentDatabase) -> AttributeEditProxy<'a> {
        let init_rev = attribute.rev;
        AttributeEditProxy {
            attribute: Cow::Borrowed(attribute),
            removed: false,
            init_rev,
            db,
        }
    }

    pub(crate) fn owned(attribute: AttributeAny, db: &'a mut DocumentDatabase) -> AttributeEditProxy<'a> {
        let init_rev = attribute.rev;
        AttributeEditProxy {
            attribute: Cow::Owned(attribute),
            removed: false,
            init_rev,
            db,
        }
    }

    pub fn finish(self) -> Option<AttributeChange> {
        if self.removed {
            Some(AttributeChange::Removed)
        } else {
            match self.attribute {
                Cow::Borrowed(_) => None,
                Cow::Owned(attr) => Some(AttributeChange::Modified(attr)),
            }
        }
    }

    pub(crate) fn changed(&mut self) -> bool {
        self.attribute.rev > self.init_rev
    }

    /// Sets a metadata entry on this attribute.
    pub fn set_metadata<T>(&mut self, metadata: Metadata<T>, value: T) -> &mut Self {
        assert!(!self.removed());
        todo!();
        self.attribute.rev += 1;
        self
    }

    /// Removes a metadata entry.
    pub fn unset_metadata<T>(&mut self, metadata: Metadata<T>) -> &mut Self {
        assert!(!self.removed());
        todo!();
        self.attribute.rev += 1;
        self
    }

    fn set_value_internal(&mut self, value: Option<Value>) -> anyhow::Result<()> {
        assert!(!self.removed());
        // problem: we're calling set_attribute_value, but the attribute might have just been created,
        // and not inserted into the database yet (and thus, would be without an identity)
        self.db.set_attribute_value(self.attribute.id, value.clone());
        self.attribute.to_mut().value = value;
        //self.attribute.rev += 1;
        Ok(())
    }

    fn set_connection_internal(&mut self, connection: Option<Path>) -> anyhow::Result<()> {
        assert!(!self.removed());
        self.db.set_attribute_connection(self.attribute.id, connection.clone());
        self.attribute.to_mut().connection = connection;
        //self.attribute.rev += 1;
        Ok(())
    }

    /// Unsets the value of the attribute.
    pub fn unset_value(&mut self) -> anyhow::Result<()> {
        self.set_value_internal(None)
    }

    /// Sets the value of the attribute.
    pub fn set_value(&mut self, value: Value) -> anyhow::Result<()> {
        self.set_value_internal(Some(value))
    }

    /// Sets the connected attribute.
    pub fn set_connection(&mut self, source: Path) -> anyhow::Result<()> {
        self.set_connection_internal(Some(source))?;
        Ok(())
    }

    pub fn unset_connection(&mut self) -> anyhow::Result<()> {
        self.set_connection_internal(None)?;
        Ok(())
    }

    pub fn remove(&mut self) {
        self.removed = true;
        //self.attribute.id = 0;
        // still increase rev counter so that the parent node detects the change
        //self.attribute.rev += 1;
    }

    pub(crate) fn removed(&self) -> bool {
        self.attribute.id == 0
    }
}

impl<'a> kyute::ToMemoizeArg for AttributeEditProxy<'a> {
    type Target = (i64, u64);
    fn to_memoize_arg(&self) -> Self::Target {
        (self.id, self.rev)
    }
}
*/
