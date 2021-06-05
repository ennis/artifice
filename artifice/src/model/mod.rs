use serde_json::json;
use std::{fmt, fmt::Formatter, ops::Deref, sync::Arc};
use string_cache::DefaultAtom;
use druid::Lens;

/// Atom (interned string used for names)
#[derive(Clone, Debug, Eq, PartialEq, Default, serde::Serialize)]
pub struct Atom(DefaultAtom);

impl Deref for Atom {
    type Target = DefaultAtom;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl druid::Data for Atom {
    fn same(&self, other: &Self) -> bool {
        &self.0 == &other.0
    }
}

impl<T> From<T> for Atom
where
    DefaultAtom: From<T>,
{
    fn from(value: T) -> Self {
        Atom(DefaultAtom::from(value))
    }
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Helper function to adjust a name so that it doesn't clash with existing names.
pub fn make_unique_name<'a>(base_name: Atom, existing: impl Iterator<Item = &'a Atom> + Clone) -> Atom {
    let mut counter = 0;
    let mut disambiguated_name = base_name.clone();

    'check: loop {
        let existing = existing.clone();
        // check for property with the same name
        for name in existing {
            if name == &disambiguated_name {
                disambiguated_name = Atom::from(format!("{}_{}", base_name, counter));
                counter += 1;
                // restart check
                continue 'check;
            }
        }
        break;
    }


    disambiguated_name
}

/// Nodes
#[derive(Clone, Debug, druid::Data, druid::Lens)]
pub struct Node {
    /// Name of this node
    //#[lens(name = "name_lens")]
    pub name: Atom,

    /// Properties
    #[lens(name = "properties_lens")]
    pub properties: Arc<Vec<Property>>,

    /// Child nodes
    pub children: Arc<Vec<Node>>,
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

/// Node property.
#[derive(Clone, Debug, serde::Serialize, druid::Data, druid::Lens)]
pub struct Property {
    /// Name of the property. Unique among all properties of a node.
    #[lens(name = "name_lens")]
    name: Atom,

    /// Type identifier
    #[lens(name = "ty_lens")]
    ty: Atom,

    /// Value of the property. It may be null if
    #[lens(name = "value_lens")]
    #[data(same_fn = "PartialEq::eq")]
    value: serde_json::Value,
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

/// A hierarchical set of interconnected nodes.
#[derive(Clone, Debug, druid::Data, druid::Lens)]
pub struct Network {
    #[lens(name = "root_lens")]
    pub root: Node,

    // current selection
    #[lens(name = "selection_lens")]
    pub selection: Arc<Vec<Node>>,
}

impl Network {
    /// Creates a new, empty network.
    pub fn new() -> Network {
        let root = Node::new();
        Network { root, selection: Arc::new(Vec::new())  }
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    /*pub fn from_json(json: serde_json::Value) -> Result<Composition, anyhow::Error> {
        let mut comp = Composition::new();
        comp.merge_from_json(json);
        Ok(comp)
    }*/

    /// Serializes this node to JSON.
    pub fn to_json(&self) -> serde_json::Value {
        self.root.to_json()
    }
}
