#![feature(const_fn_trait_bound)]

pub mod buffer;
pub mod context;
pub mod descriptor;
pub mod frame;
pub mod image;
//pub mod pipeline;
pub mod arguments;
mod device;
pub mod sampler;
pub mod shader;
pub mod utils;
pub mod vertex;

// macro support
extern crate self as mlr;

pub use arguments::{
    ArgumentBlock, Arguments, CombinedImageSampler2D, DescriptorBinding, SampledImage2D,
    UniformBuffer, ResourceVisitor, ResourceHolder
};
pub use graal::vk;
pub use mlr_macros::{Arguments, StructLayout, VertexData};
pub use vertex::{VertexAttribute, VertexData};
pub use context::ContextResources;
