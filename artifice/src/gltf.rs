//! GLTF loader

use crate::{
    Geometry, GeometryCache, GeometryId, GeometrySource, GeometrySources, Indices, Texcoords,
};
use gltf::mesh::util::ReadIndices;
use std::fmt::Display;
use std::path::Path;
use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    GltfError(gltf::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
    }
}

impl error::Error for Error {}

impl From<gltf::Error> for Error {
    fn from(e: gltf::Error) -> Self {
        Error::GltfError(e)
    }
}

struct GltfGeometrySource;

impl GeometrySource for GltfGeometrySource {
    fn load(&self, self_id: GeometryId, cache: &GeometryCache) {
        unimplemented!()
    }
}

pub fn load_gltf(
    path: &Path,
    geom_sources: &mut GeometrySources,
    geom_cache: &mut GeometryCache,
) -> Result<Vec<GeometryId>, Error> {
    let (doc, buffers, images) = gltf::import(path)?;

    let mut geom_ids = Vec::new();

    // for now only load the meshes
    for m in doc.meshes() {
        for p in m.primitives() {
            let reader = p.reader(|buffer| Some(&buffers[buffer.index()]));

            let positions: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
            let normals: Vec<[f32; 3]> = reader.read_normals().unwrap().collect();
            let tangents: Vec<[f32; 4]> = reader.read_tangents().unwrap().collect();
            let texcoords: Option<Texcoords> = reader
                .read_tex_coords(0)
                .map(|texcoords| Texcoords::F32(texcoords.into_f32().collect()));
            let indices: Option<Indices> = reader.read_indices().map(|indices| match indices {
                ReadIndices::U8(v) => Indices::U8(v.collect()),
                ReadIndices::U16(v) => Indices::U16(v.collect()),
                ReadIndices::U32(v) => Indices::U32(v.collect()),
            });

            let geom = Geometry {
                positions,
                normals,
                tangents,
                texcoords,
                indices,
            };

            let id = geom_sources.add(Box::new(GltfGeometrySource));
            geom_cache.add(id, geom);
            geom_ids.push(id);
        }
    }

    Ok(geom_ids)
}
