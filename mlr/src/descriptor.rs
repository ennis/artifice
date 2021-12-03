use crate::{
    context::{Context, ContextResources, PassBuilder},
    device::Device,
    image::ImageAny,
    vk::{DescriptorType, ShaderStageFlags},
};
use graal::vk;
use graal_spirv as spirv;
use mlr::sampler::SamplerType;
use slotmap::SlotMap;
use std::{any::TypeId, cell::Cell, collections::HashMap, mem, sync::Arc};

//-----------------------------------------------------------------------------------------
const DESCRIPTOR_POOL_PER_TYPE_COUNT: u32 = 1024;
const DESCRIPTOR_POOL_SET_COUNT: u32 = DESCRIPTOR_POOL_PER_TYPE_COUNT;

slotmap::new_key_type! {
    pub struct PipelineLayoutId;
    pub struct DescriptorSetLayoutId;
}

/// A descriptor set that may be in use.
#[derive(Copy, Clone, Debug)]
struct TrackedDescriptorSet {
    /// vulkan handle
    handle: vk::DescriptorSet,
    /// The frame that last used this descriptor
    frame: u64,
}

/// Allocator for descriptor sets of a specific layout.
#[derive(Debug)]
pub struct DescriptorSetAllocator {
    pub(crate) layout: vk::DescriptorSetLayout,
    /// The "canonical" update template for the layout.
    update_template: Option<vk::DescriptorUpdateTemplate>,
    pool_size_count: u32,
    pool_sizes: [vk::DescriptorPoolSize; 16],
    full_pools: Vec<vk::DescriptorPool>,
    ///
    pool: Option<vk::DescriptorPool>,
    /// Descriptor sets not currently in use.
    free: Vec<vk::DescriptorSet>,
}

impl DescriptorSetAllocator {
    pub fn new(
        device: &graal::ash::Device,
        descriptor_set_layout_bindings: &[vk::DescriptorSetLayoutBinding],
        update_template_entries: Option<&[vk::DescriptorUpdateTemplateEntry]>,
    ) -> DescriptorSetAllocator {
        // --- create layout
        let layout_handle = unsafe {
            let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
                binding_count: descriptor_set_layout_bindings.len() as u32,
                p_bindings: descriptor_set_layout_bindings.as_ptr(),
                ..Default::default()
            };
            device
                .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                .expect("failed to create descriptor set layout")
        };

        // --- optional: create canonical update template
        let update_template = if let Some(update_template_entries) = update_template_entries {
            unsafe {
                let descriptor_update_template_create_info =
                    vk::DescriptorUpdateTemplateCreateInfo {
                        flags: vk::DescriptorUpdateTemplateCreateFlags::empty(),
                        descriptor_update_entry_count: update_template_entries.len() as u32,
                        p_descriptor_update_entries: update_template_entries.as_ptr(),
                        template_type: vk::DescriptorUpdateTemplateType::DESCRIPTOR_SET,
                        descriptor_set_layout: Default::default(),
                        pipeline_bind_point: Default::default(),
                        pipeline_layout: Default::default(),
                        set: 0,
                        ..Default::default()
                    };
                let update_template = device
                    .create_descriptor_update_template(
                        &descriptor_update_template_create_info,
                        None,
                    )
                    .expect("failed to create descriptor update template");
                Some(update_template)
            }
        } else {
            None
        };

        let mut pool_sizes: [vk::DescriptorPoolSize; 16] = Default::default();
        // count the number of each type of descriptor
        let mut sampler_desc_count = 0;
        let mut combined_image_sampler_desc_count = 0;
        let mut sampled_image_desc_count = 0;
        let mut storage_image_desc_count = 0;
        let mut uniform_texel_buffer_desc_count = 0;
        let mut storage_texel_buffer_desc_count = 0;
        let mut uniform_buffer_desc_count = 0;
        let mut storage_buffer_desc_count = 0;
        let mut uniform_buffer_dynamic_desc_count = 0;
        let mut storage_buffer_dynamic_desc_count = 0;
        let mut input_attachment_desc_count = 0;
        let mut acceleration_structure_desc_count = 0;

        for b in descriptor_set_layout_bindings.iter() {
            match b.descriptor_type {
                vk::DescriptorType::SAMPLER => sampler_desc_count += 1,
                vk::DescriptorType::COMBINED_IMAGE_SAMPLER => {
                    combined_image_sampler_desc_count += 1
                }
                vk::DescriptorType::SAMPLED_IMAGE => sampled_image_desc_count += 1,
                vk::DescriptorType::STORAGE_IMAGE => storage_image_desc_count += 1,
                vk::DescriptorType::UNIFORM_TEXEL_BUFFER => uniform_texel_buffer_desc_count += 1,
                vk::DescriptorType::STORAGE_TEXEL_BUFFER => storage_texel_buffer_desc_count += 1,
                vk::DescriptorType::UNIFORM_BUFFER => uniform_buffer_desc_count += 1,
                vk::DescriptorType::STORAGE_BUFFER => storage_buffer_desc_count += 1,
                vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC => {
                    uniform_buffer_dynamic_desc_count += 1
                }
                vk::DescriptorType::STORAGE_BUFFER_DYNAMIC => {
                    storage_buffer_dynamic_desc_count += 1
                }
                vk::DescriptorType::INPUT_ATTACHMENT => input_attachment_desc_count += 1,
                vk::DescriptorType::ACCELERATION_STRUCTURE_KHR => {
                    acceleration_structure_desc_count += 1
                }
                _ => {}
            }
        }

        let mut pool_size_count = 0;
        if sampler_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::SAMPLER;
            pool_sizes[pool_size_count].descriptor_count =
                sampler_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if combined_image_sampler_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::COMBINED_IMAGE_SAMPLER;
            pool_sizes[pool_size_count].descriptor_count =
                combined_image_sampler_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if sampled_image_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::SAMPLED_IMAGE;
            pool_sizes[pool_size_count].descriptor_count =
                sampled_image_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if storage_image_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::STORAGE_IMAGE;
            pool_sizes[pool_size_count].descriptor_count =
                storage_image_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if uniform_texel_buffer_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::UNIFORM_TEXEL_BUFFER;
            pool_sizes[pool_size_count].descriptor_count =
                uniform_texel_buffer_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if storage_texel_buffer_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::STORAGE_TEXEL_BUFFER;
            pool_sizes[pool_size_count].descriptor_count =
                storage_texel_buffer_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if uniform_buffer_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::UNIFORM_BUFFER;
            pool_sizes[pool_size_count].descriptor_count =
                uniform_buffer_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if storage_buffer_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::STORAGE_BUFFER;
            pool_sizes[pool_size_count].descriptor_count =
                storage_buffer_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if uniform_buffer_dynamic_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC;
            pool_sizes[pool_size_count].descriptor_count =
                uniform_buffer_dynamic_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if storage_buffer_dynamic_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::STORAGE_BUFFER_DYNAMIC;
            pool_sizes[pool_size_count].descriptor_count =
                storage_buffer_dynamic_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if input_attachment_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::INPUT_ATTACHMENT;
            pool_sizes[pool_size_count].descriptor_count =
                input_attachment_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }
        if acceleration_structure_desc_count != 0 {
            pool_sizes[pool_size_count].ty = vk::DescriptorType::ACCELERATION_STRUCTURE_KHR;
            pool_sizes[pool_size_count].descriptor_count =
                acceleration_structure_desc_count * DESCRIPTOR_POOL_PER_TYPE_COUNT;
            pool_size_count += 1;
        }

        DescriptorSetAllocator {
            layout: layout_handle,
            pool_sizes,
            pool_size_count: pool_size_count as u32,
            full_pools: vec![],
            pool: None,
            free: vec![],
            update_template,
        }
    }

    /// Allocates a descriptor set.
    pub(crate) fn allocate(&mut self, device: &graal::ash::Device) -> vk::DescriptorSet {
        let handle = loop {
            let descriptor_pool = {
                if let Some(pool) = self.pool {
                    pool
                } else {
                    let pool = unsafe {
                        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo {
                            flags: vk::DescriptorPoolCreateFlags::default(),
                            max_sets: DESCRIPTOR_POOL_SET_COUNT,
                            pool_size_count: self.pool_size_count,
                            p_pool_sizes: self.pool_sizes.as_ptr(),
                            ..Default::default()
                        };
                        device
                            .create_descriptor_pool(&descriptor_pool_create_info, None)
                            .unwrap()
                    };
                    self.pool = Some(pool);
                    pool
                }
            };

            let result = unsafe {
                let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo {
                    descriptor_pool,
                    descriptor_set_count: 1,
                    p_set_layouts: &self.layout,
                    ..Default::default()
                };
                device.allocate_descriptor_sets(&descriptor_set_allocate_info)
            };

            match result {
                Ok(d) => break *d.first().unwrap(),
                Err(vk::Result::ERROR_OUT_OF_POOL_MEMORY) => {
                    // pool is full, retire the current one and loop
                    // it will allocate a new one on the next iteration
                    if let Some(pool) = mem::replace(&mut self.pool, None) {
                        self.full_pools.push(pool);
                    }
                    continue;
                }
                Err(e) => panic!("error allocating descriptor sets: {}", e),
            }
        };

        handle
    }

    /// Frees the specified descriptor set. It must have been allocated with this allocator.
    pub fn free(&mut self, ds: vk::DescriptorSet) {
        self.free.push(ds);
    }

    /// Returns the descriptor update template associated to the layout.
    pub fn update_template(&mut self) -> Option<vk::DescriptorUpdateTemplate> {
        self.update_template
    }
}

pub enum AttachmentLoadOp {
    Load,
    Clear { value: vk::ClearValue },
    DontCare,
}

pub enum AttachmentStoreOp {
    Store,
    DontCare,
}

/// Attachment description.
///
/// Specifies both the target and load/store operations.
struct AttachmentDesc<'a> {
    target: &'a mut ImageAny,
    load_op: AttachmentLoadOp,
    store_op: AttachmentStoreOp,
}

impl<'a> FragmentOutput<'a> {
    pub fn builder() -> FragmentOutputBuilder<'a> {
        FragmentOutputBuilder {
            inner: FragmentOutput {
                color_attachments: vec![],
                depth_attachment: None,
            },
        }
    }
}

/// Represents the outputs of draw pass.
pub struct FragmentOutput<'a> {
    /// Color attachment descriptions.
    color_attachments: Vec<AttachmentDesc<'a>>,
    /// Optional depth attachment description.
    depth_attachment: Option<AttachmentDesc<'a>>,
}

pub struct FragmentOutputBuilder<'a> {
    inner: FragmentOutput<'a>,
}

impl<'a> FragmentOutputBuilder<'a> {
    pub fn add_color_attachment(
        mut self,
        target: &'a mut ImageAny,
        load_op: AttachmentLoadOp,
        store_op: AttachmentStoreOp,
    ) -> Self {
        self.inner.color_attachments.push(AttachmentDesc {
            target,
            load_op,
            store_op,
        });
        self
    }

    pub fn build(self) -> FragmentOutput<'a> {
        self.inner
    }
}

const MAX_TRACKED_DESCRIPTOR_SETS: usize = 24;

/// Color attachment accessor.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ColorAttachment {
    image: graal::ImageId,
    handle: graal::vk::Image,
    format: graal::vk::Format,
    view: vk::ImageView,
}

impl ColorAttachment {
    /// Creates a new `ColorAttachment` accessor for use in the current pass.
    pub fn new(builder: &mut PassBuilder, image: &ImageAny) -> ColorAttachment {
        let stages = vk::PipelineStageFlags::FRAGMENT_SHADER;
        builder.frame.pass_image_dependency(
            image.id(),
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            stages,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );
        ColorAttachment {
            image: image.id(),
            handle: image.handle(),
            format: image.format(),
            view: Default::default(),
        }
    }

    pub(crate) fn prepare_update(&mut self, ctx: &mut ContextResources) {
        // SAFETY: TODO
        let image_view = unsafe {
            let create_info = vk::ImageViewCreateInfo {
                flags: vk::ImageViewCreateFlags::empty(),
                image: self.handle,
                view_type: vk::ImageViewType::TYPE_2D,
                format: self.format,
                components: vk::ComponentMapping::default(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: vk::REMAINING_MIP_LEVELS,
                    base_array_layer: 0,
                    layer_count: vk::REMAINING_ARRAY_LAYERS,
                },
                ..Default::default()
            };
            ctx.create_transient_image_view(&create_info)
        };

        self.view = image_view;
    }
}
