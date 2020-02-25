//! GLTF loader

use crate::document::Document;
use crate::geom::{Geometry, GeometryCache, GeometryId, GeometrySource, Indices, Texcoords};
use crate::scene::{Object, ObjectId, Scene};
use anyhow::{Error, Result};
use gltf::mesh::util::ReadIndices;
use log::warn;
use std::fmt::Display;
use std::path::Path;
use std::{error, fmt};

struct GltfGeometrySource;

impl GeometrySource for GltfGeometrySource {
    fn load(&self, self_id: GeometryId, cache: &GeometryCache) {
        // nothing to do, already loaded in the cache
    }
}

/// Loads (merges) the specified GLTF file into the specified document.
pub fn load_gltf(path: &Path, document: &mut Document) -> Result<()> {
    let (doc, buffers, images) = gltf::import(path)?;

    let mut geom_ids: Vec<GeometryId> = Vec::new();
    let mut object_ids: Vec<ObjectId> = Vec::new();

    // load meshes (geometries)
    let mut num_meshes = 0;
    for m in doc.meshes() {
        for p in m.primitives() {
            let reader = p.reader(|buffer| Some(&buffers[buffer.index()]));

            let positions: Vec<[f32; 3]> = if let Some(pos) = reader.read_positions() {
                pos.collect()
            } else {
                warn!("mesh has no positions");
                continue;
            };

            let normals: Option<Vec<[f32; 3]>> =
                reader.read_normals().map(|normals| normals.collect());
            let tangents: Option<Vec<[f32; 4]>> =
                reader.read_tangents().map(|tangents| tangents.collect());

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

            let id = document.geom_sources.add(Box::new(GltfGeometrySource));
            document.geom_cache.add(id, geom);
            geom_ids.push(id);
        }
        num_meshes += 1;
    }
    eprintln!("loaded {} geometries", num_meshes);

    // load scene nodes
    let mut scene = Scene::new();
    let mut num_nodes = 0;
    for n in doc.nodes() {
        let index = n.index();
        // create a name for the node if it has none
        let name = n
            .name()
            .map_or_else(|| format!("node_{}", index), |name| name.to_owned());
        let transform = n.transform();
        // get the corresponding geometry
        let geom = n.mesh().map(|mesh| geom_ids[mesh.index()]);

        // create and insert the object in the scene
        let obj = Object {
            name,
            transform,
            geom,
            parent: None,
            children: Vec::new(),
        };
        eprintln!("object: {}", obj.name);
        let id = scene.objects.insert(obj);
        object_ids.push(id);
        num_nodes += 1;
    }
    eprintln!("loaded {} scene objects", num_nodes);

    // build scene hierarchy
    for n in doc.nodes() {
        let children: Vec<_> = n.children().map(|c| object_ids[c.index()]).collect();

        let obj_id = object_ids[n.index()];

        for c in children.iter() {
            scene.objects.get_mut(*c).unwrap().parent = Some(obj_id);
        }

        scene.objects.get_mut(obj_id).unwrap().children = children;
    }

    Ok(())
}
