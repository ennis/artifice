use crate::{
    data::{
        atom::{make_unique_name, Atom},
        property::Property,
    },
    widgets::tree::TreeNodeModel,
};
use anyhow::anyhow;
use druid::{Data, Lens};
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;

pub type NodeList = Arc<Vec<Node>>;

#[derive(Debug,Error)]
#[error("invalid JSON document structure")]
pub struct InvalidDocumentStructure;

/// Nodes
#[derive(Clone, Debug, Data, Lens)]
pub struct Node {
    /// Name of this node
    //#[lens(name = "name_lens")]
    pub name: Atom,

    /// Properties
    #[lens(name = "properties_lens")]
    pub properties: Arc<Vec<Property>>,

    /// Child nodes
    pub children: NodeList,
}

impl Node {
    /// Creates a new node.
    pub fn new() -> Node {
        Node {
            name: Default::default(),
            properties: Default::default(),
            children: Default::default(),
        }
    }

    /// Returns an iterator over the properties of this node.
    pub fn properties(&self) -> impl Iterator<Item = &Property> {
        self.properties.iter()
    }

    /// Creates a new child node
    pub fn add_child(&mut self, name: Atom) {
        let unique_name = make_unique_name(name, self.children.iter().map(|p| &p.name));
        Arc::make_mut(&mut self.children).push(Node {
            name: Atom::from(unique_name),
            properties: Arc::new(vec![]),
            children: Arc::new(vec![]),
        })
    }

    /// Serializes the node to a JSON value.
    pub fn to_json(&self) -> serde_json::Value {
        let mut children_json = Vec::new();

        for node in self.children.iter() {
            children_json.push(node.to_json());
        }

        json!({
            "name": &self.name,
            //"properties": &node.properties,
            "children": children_json,
        })
    }

    /// Creates a node by deserializing a JSON value.
    pub fn from_json(json: &serde_json::Value) -> anyhow::Result<Node> {
        let json_obj = json.as_object().ok_or(InvalidDocumentStructure)?;
        let name = json_obj
            .get("name")
            .ok_or(InvalidDocumentStructure)?
            .as_str()
            .ok_or(InvalidDocumentStructure)?;
        let children_json = json_obj
            .get("children")
            .ok_or(InvalidDocumentStructure)?
            .as_array()
            .ok_or(InvalidDocumentStructure)?;
        let mut children = Vec::new();
        for child in children_json.iter() {
            children.push(Node::from_json(child)?);
        }

        Ok(Node {
            name: Atom::from(name),
            properties: Arc::new(vec![]),
            children: Arc::new(children),
        })
    }

    /// Adds a property to this node
    pub fn add_property(&mut self, name: Atom, ty: Atom) -> Atom {
        let unique_name = make_unique_name(name, self.properties.iter().map(|p| &p.name));

        Arc::make_mut(&mut self.properties).push(Property {
            name: unique_name.clone(),
            ty,
            value: Default::default(),
        });

        unique_name
    }

    /// Returns whether this node has a property with the given name.
    pub fn has_property(&self, name: &Atom) -> bool {
        self.properties
            .iter()
            .position(|p| &p.name == name)
            .is_some()
    }

    /// Gets the property with the given name.
    pub fn property(&self, name: &Atom) -> Option<&Property> {
        self.properties.iter().find(|p| &p.name == name)
    }

    pub fn dump(&self, indent: usize) {
        println!("{:indent$}name: {}", "", self.name, indent = indent);

        {
            let indent = indent + 2;
            for p in self.properties.iter() {
                p.dump(indent);
            }
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Node::new()
    }
}

impl TreeNodeModel for Node {
    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn with_child<V, F: FnOnce(&Self) -> V>(&self, index: usize, f: F) -> V {
        f(&self.children[index])
    }

    fn with_child_mut<V, F: FnOnce(&mut Self) -> V>(&mut self, index: usize, f: F) -> V {
        let mut child = self.children[index].clone();
        let result = f(&mut child);
        // update
        if !child.same(&self.children[index]) {
            Arc::make_mut(&mut self.children)[index] = child.clone();
        }
        result
    }
}
