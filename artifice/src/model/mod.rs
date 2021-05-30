mod edit;

use string_cache::DefaultAtom;
use std::ops::Deref;
use std::cell::{Cell, RefCell, Ref, RefMut};
use serde_json::json;
use std::rc::Rc;
use std::sync::Arc;

// Want something with history?
// -> Revision<T>
// - immutable, clonable
// - internally:
//      - Arc<Log>
//      - Revision in the history stack
//      - "synthesized" value (== checkout) (optional)
// - comparison is cheap, however getting the value of a revision may not be (if it's not checked out)

pub type Atom = DefaultAtom;

#[derive(Clone,Debug)]
pub struct Node {
    /// Name of this node
    name: Atom,

    /// Properties
    properties: Arc<Vec<Property>>,

    /// Child nodes
    children: Arc<Vec<Node>>,
}


/// Helper function to adjust a name so that it doesn't clash with existing names.
fn make_unique_name<'a>(base_name: Atom, existing: impl Iterator<Item=&'a Atom> + Clone) -> Atom {
    let mut unique_name = base_name.to_string();
    let mut counter = 0;

    'check: loop {
        let existing = existing.clone();
        // check for property with the same name
        for name in existing {
            if name.deref() == &unique_name {
                unique_name = format!("{}_{}", base_name.deref(), counter);
                counter += 1;
                // restart check
                continue 'check
            }
        }
        break
    }

    Atom::from(unique_name)
}

impl Node {
    /// Creates a new node.
    pub fn new() -> Node {
        Node {
            name: Default::default(),
            properties: Default::default(),
            children: Default::default()
        }
    }

    /// Returns the name of this node.
    pub fn name(&self) -> Atom {
        self.0.borrow().name.clone()
    }

    /// Returns the child nodes.
    pub fn children(&self) -> &[Node] {
        &self.children
    }
}

impl Default for Node {
    fn default() -> Self {
        Node::new()
    }
}

impl Node {

    /// Adds a property to this node
    pub fn add_property(
        &mut self,
        name: Atom,
        ty: Atom
    ) -> Atom
    {
        let unique_name = make_unique_name(name, inner.properties.iter().map(|p| &p.name));
        
        Arc::make_mut(&mut self.properties).push(Property {
            name: unique_name.clone(),
            ty,
            value: Default::default()
        });

        unique_name
    }

    pub fn properties(&self) -> impl Iterator<Item=&Property> {
        self.properties.iter()
    }

    /// Returns whether this node has a property with the given name.
    pub fn has_property(&self, name: &Atom) -> bool {
        self.properties.iter().position(|p| &p.name == name).is_some()
    }

    /// Gets the property with the given name.
    pub fn property(&self, name: &Atom) -> Option<&Property> {
        self.properties.iter().find(|p| &p.name == name)
    }

    pub fn dump(&self, indent: usize) {
        println!("{:indent$}name: {}", "", self.name, indent=indent);

        {
            let indent = indent + 2;
            for p in self.properties.iter() {
                p.dump(indent);
            }
        }
    }
}

#[derive(Clone,Debug, serde::Serialize)]
pub struct Property {
    /// Name
    name: Atom,

    /// Type identifier
    ty: Atom,

    /// Value of the property.
    value: serde_json::Value,
}

impl Property {
    pub fn name(&self) -> &Atom {
        &self.name
    }

    pub fn type_id(&self) -> &Atom {
        &self.ty
    }

    pub fn dump(&self, indent: usize) {
        println!("{:indent$}name  : {}", "", self.name, indent=indent);
        println!("{:indent$}type  : {}", "", self.ty, indent=indent);
        println!("{:indent$}value : {:?}", "", self.value, indent=indent);
    }
}

#[derive(Clone)]
pub struct Composition {
    root: Arc<Node>,
}

impl Composition {

    /// Creates a new composition.
    pub fn new() -> Composition {
        let root = Arc::new(Node::new());
        Composition {
            root
        }
    }

    pub fn root(&self) -> &Arc<Node> {
        self.root
    }

    pub fn create_node(&mut self, parent: NodeId, name: Atom) -> NodeId {
        let name = make_unique_name(name, self.nodes.values().map(|n| &n.name));
        let id = self.nodes.insert( Node {
            name,
            properties: vec![],
            children: vec![]
        });
        let parent = self.nodes.get_mut(parent).unwrap();
        parent.children.push(id);
        id
    }

    pub fn from_json(json: serde_json::Value) -> Result<Composition, anyhow::Error> {
        let mut comp = Composition::new();
        comp.merge_from_json(json);
        Ok(comp)
    }

    pub fn merge_from_json(&mut self, json: serde_json::Value) {
        todo!()
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    pub fn nodes(&mut self) -> impl Iterator<Item=(NodeId, &Node)> {
        self.nodes.iter()
    }

    fn node_to_json_recursive(&self, node: &Node) -> serde_json::Value {
        let mut children = Vec::new();
        for &id in node.children.iter() {
            let node = self.node(id).unwrap();
            children.push(self.node_to_json_recursive(node));
        }

        json!({
            "name": &node.name,
            "properties": &node.properties,
            "children": children,
        })
    }

    pub fn to_json(&self) -> serde_json::Value {
        let json = self.node_to_json_recursive(self.nodes.get(self.root).unwrap());
        json
    }
}