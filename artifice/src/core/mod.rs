use slotmap::{new_key_type, SlotMap};

mod node;
pub use node::Node;

new_key_type! {
    /// Unique ID of a node in a node database.
    pub struct NodeId;
}

// now define the type for a database of nodes
pub struct NodeDatabase {
    // the nodes will be a primary slotmap
    nodes: SlotMap<NodeId, Node>,
}

// there will be a limited number of plug types (not extensible)
enum PlugKind {
    Primitive,
    String,
    Image,
}

// the identifier of a plug
// - cheap to copy
// - cheap to compare
// - const-friendly (no need to use lazy_static to initialize it)
// Can be combined with other plugIDs to get a child plug
// e.g. given a string plug with ID "stringPlugID", access the "length" plug by doing "stringPlugID.length"
// -> makes a path
// given a plug id, need to know its children
// child plugs are only for stuff that we want to evaluate separately from the main value of the plug,
// but that should be connected automatically at the same time of the parent plug

// => cannot contain strings, so cannot be the name of the plug
// => hash of the plug name?
// => index?
// => must also somehow communicate the type of the plug
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct PlugId {
    kind: PlugKind,
    id: u32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct IntPlug {}

impl IntPlug {
    pub const fn id(&self) -> PlugId {}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct StringPlug {}

pub const STRING_LENGTH: IntPlug = IntPlug {};

impl StringPlug {
    pub const fn id(&self) -> PlugId {
        static DERIVED: [PlugId; 1] = [STRING_LENGTH.id()];
        PlugId {
            kind: PlugKind::String,
            id: 0,
            derived: &DERIVED,
        }
    }

    pub const fn length(&self) -> IntPlug {}
}

trait Op {
    /// Returns the interface of the node.
    fn interface(&self) -> ();
}
