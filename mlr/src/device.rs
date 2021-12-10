use crate::{Context, vk};
use slotmap::{SecondaryMap, SlotMap};
use std::{
    any::TypeId,
    borrow::BorrowMut,
    cell::Cell,
    collections::HashMap,
    sync::{Arc, Mutex},
};
use std::collections::VecDeque;
use graal::descriptor::DescriptorSetAllocator;
use mlr::sampler::SamplerInner;
use crate::sampler::{Sampler, SamplerType};


// Same architecture as graal, but for everything other than buffers & images
pub(crate) struct DeviceInner {
    current_frame: graal::FrameNumber,
    descriptor_set_layouts: ObjectTracker<DescriptorSetLayoutId, vk::DescriptorSetLayout>,
    samplers: ObjectTracker<SamplerId, vk::Sampler>,
    pipeline_layouts: ObjectTracker<PipelineLayoutId, vk::PipelineLayout>,
    pipelines: ObjectTracker<PipelineId, vk::Pipeline>,
    descriptor_allocators: SecondaryMap<DescriptorSetLayoutId, DescriptorSetAllocator>,
    descriptor_set_layout_by_typeid: HashMap<TypeId, DescriptorSetLayoutId>,
    sampler_by_typeid: HashMap<TypeId, Sampler>,
}

#[derive(Clone)]
pub struct Device {
    backend: Arc<graal::Device>,
    inner: Arc<Mutex<DeviceInner>>,
}

impl Device {
    pub unsafe fn create_device_and_context(
        present_surface: Option<vk::SurfaceKHR>,
    ) -> (Device, Context) {
        let (backend_device, backend_context) =
            graal::Device::create_device_and_context(present_surface);
        (
            Device {
                inner: Arc::new(Mutex::new(DeviceInner {
                    current_frame: Default::default(),
                    descriptor_set_layouts: Default::default(),
                    descriptor_set_layout_by_typeid: Default::default(),
                    sampler_by_typeid: Default::default(),
                    samplers: Default::default(),
                    pipeline_layouts: Default::default(),
                    pipelines: Default::default(),
                    descriptor_allocators: SecondaryMap::default(),
                })),
                backend: backend_device,
            },
            Context {
                backend: backend_context,
                in_flight: VecDeque::new(),
            },
        )
    }

    /// Returns the underlying `graal::Device`.
    pub fn backend(&self) -> &Arc<graal::Device> {
        &self.backend
    }

    /// Returns the underlying Vulkan device (`ash::Device`).
    pub fn vulkan_device(&self) -> &graal::ash::Device {
        &self.backend.device
    }

    /// Creates a sampler object from a type implementing `ToSampler`.
    ///
    /// The returned object lives as long as the context is alive.
    pub fn get_or_create_sampler(
        &mut self,
        sampler: impl SamplerType,
    ) -> Sampler {
        let device = &self.backend.device;
        let mut inner = self.inner.lock().unwrap();
        let mut samplers = &mut inner.samplers;
        let mut sampler_by_typeid = &mut inner.sampler_by_typeid;
        if let Some(type_id) = sampler.unique_type_id() {
            *sampler_by_typeid
                .entry(type_id)
                .or_insert_with(|| {
                    let sampler = sampler.to_sampler(device);
                    let id = samplers.insert(sampler);
                    Sampler(Arc::new(SamplerInner {
                        device: self.clone(),
                        id,
                        sampler: Default::default()
                    })
                })
        } else {
            todo!()
        }
    }

    pub(crate) fn destroy_sampler(&self, id: SamplerId) {
        let mut inner = self.inner.lock().unwrap();
        inner.samplers.destroy_on_frame_completed(inner.current_frame, );
    }

    /// Creates a descriptor set layout.
    pub(crate) fn get_or_create_descriptor_set_layout(
        &mut self,
        type_id: Option<TypeId>,
        bindings: &[vk::DescriptorSetLayoutBinding],
        update_template_entries: Option<&[vk::DescriptorUpdateTemplateEntry]>,
    ) -> (vk::DescriptorSetLayout, DescriptorSetLayoutId) {
        let device = &self.backend.device;
        let layouts = &mut self.descriptor_set_layouts;
        let update_templates = &mut self.descriptor_update_templates;
        let id = if let Some(type_id) = type_id {
            *self
                .descriptor_set_layout_by_typeid
                .entry(type_id)
                .or_insert_with(|| unsafe {
                    // --- create layout ---
                    let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
                        binding_count: bindings.len() as u32,
                        p_bindings: bindings.as_ptr(),
                        ..Default::default()
                    };
                    let layout = device
                        .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                        .expect("failed to create descriptor set layout");

                    let layout_id = layouts.insert(layout);

                    // --- create update template ---
                    if let Some(update_template_entries) = update_template_entries {
                        unsafe {
                            let descriptor_update_template_create_info =
                                vk::DescriptorUpdateTemplateCreateInfo {
                                    flags: vk::DescriptorUpdateTemplateCreateFlags::empty(),
                                    descriptor_update_entry_count: update_template_entries.len()
                                        as u32,
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
                            update_templates.insert(layout_id, update_template);
                        }
                    }
                    layout_id
                })
        } else {
            todo!()
        };

        (*self.descriptor_set_layouts.get(id).unwrap(), id)
    }

    /// Creates an argument block
    pub fn create_argument_block<T: Arguments>(&mut self, mut args: T) -> ArgumentBlock<T> {
        let (_, layout_id) = self.get_or_create_descriptor_set_layout(
            args.unique_type_id(),
            args.get_descriptor_set_layout_bindings(),
            args.get_descriptor_set_update_template_entries(),
        );
        //let allocator = self.descriptor_allocators.get_mut(layout_id).unwrap();
        let update_template = self.descriptor_update_templates.get();
        let descriptor_set = allocator.allocate(&self.device.device);

        /*self.frame_resources
            .descriptor_sets
            .push((layout_id, descriptor_set));

        // SAFETY: TODO?
        unsafe {
            args.update_descriptor_set(self, descriptor_set, update_template);
        }*/

        // FIXME: we have to track usages of descriptor set layouts, descriptor sets,

        ArgumentBlock {
            args,
            set_layout_id: Default::default(),
            set_layout: Default::default(),
            update_template: Default::default(),
            descriptor_set,
        }
    }

    /// Returns the descriptor set allocator for the given layout id.
    pub fn get_or_create_descriptor_set_layout_for_type<T: StaticArguments>(
        &mut self,
    ) -> vk::DescriptorSetLayout {
        self.get_or_create_descriptor_set_layout(
            Some(T::TYPE_ID),
            T::LAYOUT,
            T::UPDATE_TEMPLATE_ENTRIES,
        )
        .0
    }
}
