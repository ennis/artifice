use crate::{
    descriptor::{
        DescriptorSetAllocator, DescriptorSetLayoutCache, DescriptorSetLayoutId, FragmentOutput,
    },
    frame::Frame,
    pipeline::GraphicsPipeline,
    shader::Batch,
};
use graal::{vk, GpuFuture};
use slotmap::SlotMap;
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// Transient objects that should be deleted or recycled once the frame has completed execution.
struct FrameResources {
    future: GpuFuture,
    descriptor_sets: Vec<vk::DescriptorSet>,
    framebuffers: Vec<vk::Framebuffer>,
    image_views: Vec<vk::ImageView>,
}

/// MLR context.
#[derive(Clone)]
pub struct Context {
    pub(crate) context: graal::Context,
    pub(crate) arena: bumpalo::Bump,
    pub(crate) descriptors: DescriptorSetLayoutCache,
}

impl Context {
    /// Creates a new context.
    pub fn new(device: graal::Device) -> Context {
        let context = graal::Context::with_device(device);
        let device = context.device().clone();
        Context {
            context,
            arena: Default::default(),
            descriptors: DescriptorSetLayoutCache::new(device),
        }
    }

    /// Returns a reference to the underlying `graal::Device`
    pub fn device(&self) -> &Arc<graal::Device> {
        self.context.device()
    }

    /// Returns a reference to the underlying `VkDevice`
    pub fn vulkan_device(&self) -> &graal::ash::Device {
        &self.context.device().device
    }
}
