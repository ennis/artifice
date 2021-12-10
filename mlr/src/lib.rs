#![feature(const_fn_trait_bound)]

pub mod buffer;
pub mod context;
pub mod frame;
pub mod image;
//pub mod pipeline;
pub mod arguments;
mod device;
pub mod sampler;
pub mod shader;
pub mod utils;
pub mod vertex;
pub mod pipeline;

// macro support
extern crate self as mlr;

pub use arguments::{
    ArgumentBlock, Arguments, CombinedImageSampler2D, DescriptorBinding, ResourceHolder,
    ResourceVisitor, SampledImage2D, UniformBuffer,
};
pub use context::{
    AttachmentLoadOp,
    AttachmentStoreOp, Context, Device, Frame, RenderPass,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
};
pub use graal::vk;
pub use mlr_macros::{Arguments, StructLayout, VertexData};
pub use vertex::{VertexAttribute, VertexData};
