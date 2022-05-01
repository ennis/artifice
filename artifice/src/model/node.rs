use crate::model::{
    attribute::{Attribute, AttributeAny},
    Atom, FromValue, Metadata, ModelPath, NamedObject, Value,
};
use imbl::{HashMap, Vector};
use kyute::Data;
use std::sync::Arc;

/// Nodes.
#[derive(Clone, Debug)]
pub struct Node {
    /// Common properties of named objects.
    pub base: NamedObject,
    /// Child nodes.
    pub children: HashMap<Atom, Node>,
    /// Attributes.
    pub attributes: HashMap<Atom, AttributeAny>,
    /// Node metadata.
    pub metadata: HashMap<Atom, Value>,
}

impl Node {
    /*/// Writes this node into an open database.
    pub fn write(&self, conn: &rusqlite::Connection) -> Result<()> {
        // recursively write this node and children
        self.base.write(conn)?;
        for (_, n) in self.children.iter() {
            n.write(conn);
        }
        Ok(())
    }*/

    /// Creates a new node.
    pub(crate) fn new(path: ModelPath) -> Node {
        Node {
            base: NamedObject { id: 0, path },
            children: Default::default(),
            attributes: Default::default(),
            metadata: Default::default(),
        }
    }

    /// Generates a unique child name from the specified stem.
    pub fn make_unique_child_name(&self, stem: impl Into<Atom>) -> Atom {
        let mut counter = 0;
        let stem = stem.into();
        let mut unique_name = stem.clone();

        'check: loop {
            // check for property with the same name
            for node in self.children.values() {
                if node.base.path.name() == unique_name {
                    unique_name = Atom::from(format!("{}_{}", stem, counter));
                    counter += 1;
                    // restart check
                    continue 'check;
                }
            }
            break;
        }

        unique_name
    }

    /// Finds a child node by name.
    pub fn find_child(&self, name: &Atom) -> Option<&Node> {
        self.children.get(name)
    }

    /// Finds a child node by name and returns a mutable reference to it.
    pub fn find_child_mut(&mut self, name: &Atom) -> Option<&mut Node> {
        self.children.get_mut(name)
    }

    /// Adds a child node. Used internally by `Document`.
    pub fn add_child(&mut self, name: Atom) -> &mut Node {
        let child_path = self.base.path.join(name.clone());
        self.children.entry(name).or_insert(Node::new(child_path))
    }

    pub fn find_attribute(&self, name: &Atom) -> Option<&AttributeAny> {
        self.attributes.get(name)
    }

    /// Returns a metadata value.
    pub fn metadata<T: FromValue>(&self, metadata: Metadata<T>) -> Option<T> {
        let v = self.metadata.get(&Atom::from(metadata.name))?;
        T::from_value(v)
    }

    // Adds an attribute.
    //pub fn add_attribute(&mut self, name: Atom)

    /// Recursively dumps the structure of this node and its children to the standard output.
    pub fn dump(&self, indent: usize) {
        let name = self.base.path.name();

        println!(
            "{:indent$}{}",
            "",
            if name.is_empty() { "<root>" } else { &name },
            indent = indent
        );

        {
            let indent = indent + 2;
            for n in self.children.values() {
                n.dump(indent);
            }
        }
    }
}

impl Data for Node {
    fn same(&self, other: &Self) -> bool {
        self.base.same(&other.base) && self.children.ptr_eq(&other.children)
    }
}
