use anyhow::Error;
use crate::model::atom::Atom;
use kyute::Data;
use crate::json;
use crate::model::{NamedObject, Value};

/// Node property. Has a type and a current value.
#[derive(Clone, Debug, Data)]
pub struct Property {
    /// Base named object.
    pub base: NamedObject,

    /// Type identifier
    pub ty: Atom,

    /// Value of the property.
    pub value: Value,
}

impl Property {

    /// Name of the property
    pub fn name(&self) -> &Atom {
        &self.name
    }

    /// Type ID of the property
    pub fn type_id(&self) -> &Atom {
        &self.ty
    }

    /*pub fn dump(&self, indent: usize) {
        println!("{:indent$}name  : {}", "", self.name, indent = indent);
        println!("{:indent$}type  : {}", "", self.ty, indent = indent);
        println!("{:indent$}value : {:?}", "", self.value, indent = indent);
    }*/
}
