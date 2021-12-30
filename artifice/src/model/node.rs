use crate::model::{
    atom::{make_unique_name, Atom},
    property::Property,
    NamedObject,
};
use imbl::HashMap;
use kyute::Data;
use std::sync::Arc;

/// Nodes.
#[derive(Clone, Debug)]
pub struct Node {
    pub base: NamedObject,
    pub children: HashMap<Atom, Node>,
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

    /// Finds a child node by name.
    pub fn find_child(&self, name: &Atom) -> Option<&Node> {
        self.children.get(name)
    }

    /// Finds a child node by name and returns a mutable reference to it.
    pub fn find_child_mut(&mut self, name: &Atom) -> Option<&mut Node> {
        self.children.get_mut(name)
    }

    /// Adds a child node. Used internally by `Document`.
    pub(crate) fn add_child(&mut self, node: Node) {
        self.children.insert_mut(node.base.path.name(), node);
    }

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
