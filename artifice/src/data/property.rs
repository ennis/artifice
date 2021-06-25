use crate::data::atom::Atom;
use druid::{Data, Lens};
use crate::data::Value;

/// Node property.
#[derive(Clone, Debug, Data, Lens)]
pub struct Property {
    /// Name of the property. Unique among all properties of a node.
    #[lens(name = "name_lens")]
    pub name: Atom,

    /// Type identifier
    #[lens(name = "ty_lens")]
    pub ty: Atom,

    /// Value of the property.
    #[lens(name = "value_lens")]
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

    pub fn dump(&self, indent: usize) {
        println!("{:indent$}name  : {}", "", self.name, indent = indent);
        println!("{:indent$}type  : {}", "", self.ty, indent = indent);
        println!("{:indent$}value : {:?}", "", self.value, indent = indent);
    }
}
