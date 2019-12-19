use slotmap::new_key_type;
use slotmap::SecondaryMap;
use slotmap::SlotMap;

pub use veda;

pub mod gltf;

new_key_type! {
    /// ID for geometry data. Uniquely identifies:
    /// - a geometry source
    /// - its associated entry in the main-memory geometry cache
    /// - its associated entry in the GPU-memory geometry cache
    pub struct GeometryId;
}

pub struct GeometrySources {
    sources: SlotMap<GeometryId, Box<dyn GeometrySource>>,
}

impl GeometrySources {
    pub fn add(&mut self, source: Box<dyn GeometrySource>) -> GeometryId {
        self.sources.insert(source)
    }
}

pub enum Indices {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
}

pub enum Texcoords {
    F32(Vec<[f32; 2]>),
}

// A geometry represents the in-memory geometry data of a single object
pub struct Geometry {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    tangents: Vec<[f32; 4]>,
    texcoords: Option<Texcoords>,
    indices: Option<Indices>,
}

/// Geometry data cache in main memory.
pub struct GeometryCache {
    entries: SecondaryMap<GeometryId, Geometry>,
}

impl GeometryCache {
    pub fn add(&mut self, id: GeometryId, data: Geometry) {
        self.entries.insert(id, data);
    }
}

pub trait GeometrySource {
    /// Loads geometry data in memory.
    fn load(&self, self_id: GeometryId, cache: &GeometryCache);
}
