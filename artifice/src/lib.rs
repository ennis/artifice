// macro support
extern crate self as artifice;

pub mod util;
pub mod core;

// Layer 1 - Persistence to the disk, undo/redo
//

// - Entities are fundamental
// - Entities exist in a Database
// ? Only one type of entity can be stored in a database
// - Components attached to entities
// - Can remove entities, and all components are removed when an entity is removed
// - Components are stored within the database
// - Schemas correspond to a set of components attached to an entity
// - Components can be unsized types and trait objects (dyn Trait)
// - There can be only one instance of a component type on an entity
//
// * Assume that every operation can be done by an end-user
// * Every transaction should result in a valid data model state
// * The database should be easily introspectable
//
// - The basic operations are:
//      - Create/Delete an entity
//      - Create an entity from a schema
//      - Add a component to an entity
//      - Remove a component
//      - Modify a component
//
// - Databases should maintain coherence:
//      - Assume that a component refers to two other entities (relationship)
//      - If one of the entities is removed, remove the component
//      - (DB: on delete cascade)
//
// - Components are Objects
// - Objects can be reflected:
//      - Iterate over fields
//
// Undo/redo should be supported without too much extra code
//
// e.g. component.set_xxx(value, &mut edit)

// problem: representing an operation on a complex data model in an undo command
// - path = value
// If directly modifying the value through a mut reference, there's no way of preserving the
// previous state.
// Lenses, possibly?

// "Lightweight lenses"
// == a string
// .name => lookup by name
// .<number> => index-based lookup

// root.nodes.[name].value = ...;
// will be converted to

// let nodes = root.lookup_field("nodes")?;
// let v = nodes.lookup_entry(name)?;
// let value = v.lookup_field(value)?;
// value.set(&v)

