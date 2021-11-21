use crate::{
    descriptor::{DescriptorSetAllocator, DescriptorSetLayoutId, FragmentOutput},
    pipeline::GraphicsPipeline,
};
use slotmap::SlotMap;
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// MLR context.
#[derive(Clone)]
pub struct Context {
    pub(crate) context: graal::Context,
    pub(crate) descriptor_set_allocators: SlotMap<DescriptorSetLayoutId, DescriptorSetAllocator>,
    pub(crate) descriptor_set_layout_by_typeid: HashMap<TypeId, DescriptorSetLayoutId>,

}

impl Context {
    /// Creates a new context.
    pub fn new(device: graal::Device) -> Context {
        let context = graal::Context::with_device(device);
        Context {
            context,
            descriptor_set_allocators: SlotMap::with_key(),
            descriptor_set_layout_by_typeid: Default::default(),
        }
    }

    /// Returns a reference to the underlying `graal::Device`
    pub fn device(&self) -> &Arc<graal::Device> {
        self.context.device()
    }

    /// Returns a reference to the underlying `VkDevice`
    pub fn vulkan_device(&self) -> &Arc<graal::Device> {
        self.context.device()
    }
}

/*impl Context {
    /// Root draw command
    pub fn draw(
        stages: &ShaderStages,
        pipeline_params: &GraphicsPipelineParameters,

        fragment_output: &FragmentOutput,
    ) {
    }
}
*/