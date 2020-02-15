use slotmap::{SecondaryMap, SlotMap};

slotmap::new_key_type! {
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
    pub fn new() -> GeometrySources {
        GeometrySources {
            sources: SlotMap::with_key(),
        }
    }

    pub fn add(&mut self, source: Box<dyn GeometrySource>) -> GeometryId {
        self.sources.insert(source)
    }
}

#[derive(Clone, Debug)]
pub enum Indices {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
}

#[derive(Clone, Debug)]
pub enum Texcoords {
    F32(Vec<[f32; 2]>),
}

// A geometry represents the in-memory geometry data of a single object
#[derive(Clone, Debug)]
pub struct Geometry {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 4]>,
    pub texcoords: Option<Texcoords>,
    pub indices: Option<Indices>,
}

/// Geometry data cache in main memory.
#[derive(Clone, Debug)]
pub struct GeometryCache {
    entries: SecondaryMap<GeometryId, Geometry>,
}

impl GeometryCache {
    /// Creates a new, empty geometry cache.
    pub fn new() -> GeometryCache {
        GeometryCache {
            entries: SecondaryMap::new(),
        }
    }

    pub fn add(&mut self, id: GeometryId, data: Geometry) {
        self.entries.insert(id, data);
    }
}

pub trait GeometrySource {
    /// Loads geometry data in memory.
    fn load(&self, self_id: GeometryId, cache: &GeometryCache);
}
