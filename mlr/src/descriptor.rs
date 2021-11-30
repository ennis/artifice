use crate::{context::Context, image::ImageAny, shader::ShaderArguments, Buffer, ContextCache};
use graal::vk;
use graal_spirv as spirv;
use slotmap::SlotMap;
use std::{any::TypeId, collections::HashMap, mem, sync::Arc};

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
    layout: vk::DescriptorSetLayout,
    /// The "canonical" update template for the layout.
    update_template: Option<vk::DescriptorUpdateTemplate>,
    pool_size_count: u32,
    pool_sizes: [vk::DescriptorPoolSize; 16],
    full_pools: Vec<vk::DescriptorPool>,
    ///
    pool: Option<vk::DescriptorPool>,
    /// Descriptor sets not currently in use.
    free: Vec<vk::DescriptorSet>,
    /// Descriptor sets that may be in use.
    used: Vec<TrackedDescriptorSet>,
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
            used: vec![],
            update_template,
        }
    }

    /// Gets a descriptor set.
    pub(crate) fn allocate_set(
        &mut self,
        device: &graal::ash::Device,
        frame: u64,
    ) -> vk::DescriptorSet {
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

        self.used.push(TrackedDescriptorSet { handle, frame });
        handle
    }

    /// Recycles all descriptor sets that are not in use anymore, given new completed serials.
    // TODO "recycle" instead? it doesn't actually free memory
    pub fn recycle(&mut self, completed_frame: u64) {
        let mut i = 0;
        while i < self.used.len() {
            if self.used[i].frame <= completed_frame {
                self.free.push(self.used.swap_remove(i).handle);
            } else {
                i += 1;
            }
        }
    }
}

/// Cache that holds descriptor set layouts.
///
/// You can associate a layout to a typeid and query it by typeid.
pub struct DescriptorSetLayoutCache {
    device: Arc<graal::Device>,
    allocators: SlotMap<DescriptorSetLayoutId, DescriptorSetAllocator>,
    by_typeid: HashMap<TypeId, DescriptorSetLayoutId>,
}

impl DescriptorSetLayoutCache {
    /// Creates a new, empty `DescriptorSetLayoutCache`.
    pub fn new(device: Arc<graal::Device>) -> DescriptorSetLayoutCache {
        DescriptorSetLayoutCache {
            device,
            allocators: SlotMap::with_key(),
            by_typeid: Default::default(),
        }
    }

    /// Creates a descriptor set layout and an associated allocator.
    pub fn get_or_create_descriptor_set_layout(
        &mut self,
        type_id: Option<TypeId>,
        bindings: &[vk::DescriptorSetLayoutBinding],
        update_template_entries: Option<&[vk::DescriptorUpdateTemplateEntry]>,
    ) -> (vk::DescriptorSetLayout, DescriptorSetLayoutId) {
        let device = &self.device.device;
        let mut allocators = &mut self.allocators;
        let id = if let Some(type_id) = type_id {
            *self.by_typeid.entry(type_id).or_insert_with(|| {
                allocators.insert(DescriptorSetAllocator::new(
                    device,
                    bindings,
                    update_template_entries,
                ))
            })
        } else {
            // no typeid, don't
            allocators.insert(DescriptorSetAllocator::new(
                device,
                bindings,
                update_template_entries,
            ))
        };

        (self.allocators.get(id).unwrap().layout, id)
    }

    /// Returns the descriptor set allocator for the given layout id.
    pub(crate) fn get_descriptor_set_allocator(
        &mut self,
        id: DescriptorSetLayoutId,
    ) -> &mut DescriptorSetAllocator {
        self.allocators.get_mut(id).unwrap()
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
