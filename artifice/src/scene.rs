//! Scenes

use crate::geom::GeometryId;
use crate::material::StandardViewportMaterial;
use gltf::scene::Transform;
use slotmap::{SecondaryMap, SlotMap};

slotmap::new_key_type! {
    pub struct ObjectId;
}

/// An object in a scene.
#[derive(Clone, Debug)]
pub struct Object {
    pub name: String,
    pub transform: Transform,
    pub geom: Option<GeometryId>,
    pub parent: Option<ObjectId>,
    pub children: Vec<ObjectId>,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub objects: SlotMap<ObjectId, Object>,
    pub std_materials: SecondaryMap<ObjectId, StandardViewportMaterial>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            objects: SlotMap::with_key(),
            std_materials: SecondaryMap::new(),
        }
    }

    pub fn add_object(&mut self, object: Object) -> ObjectId {
        self.objects.insert(object)
    }
}
