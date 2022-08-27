use crate::model::{
    metadata,
    param::{AttributeType, Param},
    Atom, Metadata, Path, Value,
};
use imbl::{HashMap, OrdMap, Vector};
use kyute::Data;
use std::{borrow::Cow, convert::TryFrom, ops::Deref, sync::Arc};

/// Nodes.
#[derive(Clone, Debug)]
pub struct Node {
    /// Revision index, incremented on every change.
    pub(crate) rev: u64,
    /// Unique node ID.
    pub id: i64,
    /// Path of this object in the document tree. Contains the name of the object.
    pub path: Path,
    /// Attributes.
    pub attributes: OrdMap<Atom, Param>,
    /// Node metadata.
    pub metadata: OrdMap<Atom, Value>,
    /// Child nodes
    pub children: OrdMap<Atom, Node>,
}

impl Node {
    /// Creates a new node.
    pub(crate) fn new(id: i64, path: Path) -> Node {
        Node {
            rev: 0,
            id,
            path,
            attributes: Default::default(),
            metadata: Default::default(),
            children: Default::default(),
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
                if node.name() == unique_name {
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

    pub fn name(&self) -> Atom {
        self.path.name()
    }

    /// Returns the attribute with the specified name.
    pub fn attribute(&self, name: &Atom) -> Option<&Param> {
        self.attributes.get(name)
    }

    /// Returns a mutable reference to the attribute with the specified name.
    pub fn attribute_mut(&mut self, name: &Atom) -> Option<&mut Param> {
        self.attributes.get_mut(name)
    }

    /// Returns a metadata value.
    ///
    /// TODO propagate the reason for the error.
    pub fn metadata<T: TryFrom<Value>>(&self, metadata: Metadata<T>) -> Option<T> {
        let v = self.metadata.get(&Atom::from(metadata.name))?;
        T::try_from(v.clone()).ok()
    }

    /// Returns the operator name associated to this node.
    pub fn operator(&self) -> Option<Atom> {
        self.metadata(metadata::OPERATOR)
    }

    pub fn child(&self, name: &Atom) -> Option<&Node> {
        self.children.get(name)
    }

    pub fn child_mut(&mut self, name: &Atom) -> Option<&mut Node> {
        self.children.get_mut(name)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Edits
////////////////////////////////////////////////////////////////////////////////////////////////////
impl Node {}

/*
#[derive(Debug)]
pub struct NodeEditProxy<'a> {
    node: Cow<'a, Node>,
    db: &'a mut DocumentDatabase,
    init_rev: u64,
    removed: bool,
}

pub enum NodeChange {
    Modified(Node),
    Removed,
}

impl<'a> Deref for NodeEditProxy<'a> {
    type Target = Node;
    fn deref(&self) -> &Self::Target {
        self.node.deref()
    }
}

impl<'a> NodeEditProxy<'a> {
    pub(crate) fn borrowed(node: &'a Node, db: &'a mut DocumentDatabase) -> NodeEditProxy<'a> {
        let init_rev = node.rev;
        NodeEditProxy {
            node: Cow::Borrowed(node),
            init_rev,
            removed: false,
            db,
        }
    }

    pub(crate) fn owned(node: Node, db: &'a mut DocumentDatabase) -> NodeEditProxy<'a> {
        let init_rev = node.rev;
        NodeEditProxy {
            node: Cow::Owned(node),
            init_rev,
            removed: false,
            db,
        }
    }

    pub fn finish(self) -> Option<NodeChange> {
        if self.removed {
            Some(NodeChange::Removed)
        } else {
            match self.node {
                Cow::Borrowed(_) => None,
                Cow::Owned(node) => Some(NodeChange::Modified(node)),
            }
        }
    }

    pub fn changed(&mut self) -> bool {
        self.node.rev > self.init_rev || self.removed
    }

    pub fn get_or_create_attribute<R>(
        &mut self,
        name: impl Into<Atom>,
        ty: impl Into<Atom>,
        value: Option<Value>,
        f: impl FnOnce(&mut AttributeEditProxy) -> R,
    ) -> R {
        let name = name.into();
        let result;

        let change = if let Some(attribute) = self.node.attribute(&name) {
            let mut proxy = AttributeEditProxy::borrowed(attribute, self.db);
            result = f(&mut proxy);
            proxy.finish()
        } else {
            let ty = ty.into();
            let path = self.node.path.join_attribute(name.clone());
            let id = self
                .db
                .insert_attribute(self.node.id, &path, ty.clone(), value.clone(), None)
                .unwrap();
            let mut proxy = AttributeEditProxy::owned(AttributeAny::new(id, path, ty, value, None), self.db);
            result = f(&mut proxy);
            proxy.finish()
        };

        match change {
            Some(AttributeChange::Modified(attr)) => {
                self.node.to_mut().attributes.insert(name, attr);
            }
            Some(AttributeChange::Removed) => {
                self.node.to_mut().attributes.remove(&name);
            }
            None => {}
        }

        result
    }

    /// Recursively edit this node's attributes.
    ///
    /// The provided closure is called with an `AttributeEditProxy` for each of this node's attributes.
    pub fn edit_attributes(&mut self, mut f: impl FnMut(&mut AttributeEditProxy)) {
        let mut changes = vec![];
        for (_, attribute) in self.node.attributes.iter() {
            let mut proxy = AttributeEditProxy::borrowed(attribute, self.db);
            f(&mut proxy);
            if let Some(change) = proxy.finish() {
                changes.push((attribute.name(), change));
            }
        }

        for (attr_name, change) in changes {
            match change {
                AttributeChange::Modified(attr) => {
                    self.node.to_mut().attributes.insert(attr_name, attr);
                }
                AttributeChange::Removed => {
                    self.node.to_mut().attributes.remove(&attr_name);
                }
            }
        }
    }

    /// Edits a child node.
    ///
    /// The provided closure is called with a `NodeEditProxy` for the child node.
    pub fn get_or_create_node<R>(&mut self, name: impl Into<Atom>, f: impl FnOnce(&mut NodeEditProxy) -> R) -> R {
        let name = name.into();
        let result;
        let change;

        if let Some(child) = self.node.child(&name) {
            let mut proxy = NodeEditProxy::borrowed(child, self.db);
            result = f(&mut proxy);
            change = proxy.finish();
        } else {
            let path = self.node.path.join(name.clone());
            let id = self.db.insert_node(self.node.id, &path).unwrap();
            let mut proxy = NodeEditProxy::owned(Node::new(id, path), self.db);
            result = f(&mut proxy);
            change = proxy.finish();
        };

        match change {
            Some(NodeChange::Modified(node)) => {
                self.node.to_mut().children.insert(name, node);
            }
            Some(NodeChange::Removed) => {
                self.node.to_mut().children.remove(&name);
            }
            None => {}
        }

        result
    }

    /// Recursively edit this node's children.
    ///
    /// The provided closure is called with an `NodeEditProxy` for each of this node's children.
    pub fn edit_children(&mut self, mut f: impl FnMut(&mut NodeEditProxy)) {
        let mut changes = vec![];
        for (_, child) in self.node.children.iter() {
            let mut proxy = NodeEditProxy::borrowed(child, self.db);
            f(&mut proxy);
            if let Some(change) = proxy.finish() {
                changes.push((child.name(), change));
            }
        }

        for (child_name, change) in changes {
            match change {
                NodeChange::Modified(node) => {
                    self.node.to_mut().children.insert(child_name, node);
                }
                NodeChange::Removed => {
                    self.node.to_mut().children.remove(&child_name);
                }
            }
        }
    }

    /// Removes this node from the document.
    pub fn remove(&mut self) {
        assert!(!self.node.path.is_root(), "cannot remove root node");
        self.db.remove_node(self.id);
        self.removed = true;
    }

    pub(crate) fn removed(&self) -> bool {
        self.removed
    }
}*/
