#![feature(specialization)]
#![feature(unsized_locals)]

// macro support
extern crate self as artifice;

pub mod application;
pub mod document;
pub mod geom;
pub mod gltf;
pub mod material;
pub mod render;
pub mod scene;
pub mod ui;
pub mod util;

pub mod model;
