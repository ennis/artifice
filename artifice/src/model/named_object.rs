use artifice::model::ModelPath;
use kyute_common::{Atom, Data};

/// Objects that belong to the document object tree, they have a name and are accessible via their path.
#[derive(Clone, Debug, Data)]
pub struct NamedObject {
    /// rowid in the `named_objects` table.
    pub id: i64,
    /// Path of this object in the document tree. Contains the name of the object.
    pub path: ModelPath,
}

impl NamedObject {
    /// Returns the name of this object.
    pub fn name(&self) -> Atom {
        self.path.name()
    }
}
