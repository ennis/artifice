use crate::{
    sampler::{Sampler, SamplerInner, SamplerType},
    vk, Arguments,
};
use mlr::arguments::StaticArguments;
use slotmap::{SecondaryMap, SlotMap};
use std::{
    any::TypeId,
    borrow::BorrowMut,
    cell::Cell,
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

pub(crate) struct DeviceInner {
    current_frame: graal::FrameNumber,
    descriptor_set_layout_by_typeid: HashMap<TypeId, vk::DescriptorSetLayout>,
    sampler_by_typeid: HashMap<TypeId, vk::Sampler>,
}

#[derive(Clone)]
pub struct Device {
    pub(crate) backend: Arc<graal::Device>,
    pub(crate) inner: Arc<Mutex<DeviceInner>>,
}

impl Device {
    /// Returns the underlying `graal::Device`.
    pub fn backend(&self) -> &Arc<graal::Device> {
        &self.backend
    }

    /// Returns the underlying Vulkan device (`ash::Device`).
    pub fn vulkan_device(&self) -> &graal::ash::Device {
        &self.backend.device
    }

    /// Creates a descriptor set layout.
    pub(crate) unsafe fn get_or_create_descriptor_set_layout(
        &self,
        type_id: Option<TypeId>,
        bindings: &[vk::DescriptorSetLayoutBinding],
    ) -> vk::DescriptorSetLayout {
        let device = &self.backend.device;
        let mut inner = self.inner.lock().unwrap();

        if let Some(type_id) = type_id {
            // look in the map to see if we have already created the layout for the given type.
            *inner
                .descriptor_set_layout_by_typeid
                .entry(type_id)
                .or_insert_with(|| unsafe {
                    // not in the map, create layout and insert it into the map
                    let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
                        binding_count: bindings.len() as u32,
                        p_bindings: bindings.as_ptr(),
                        ..Default::default()
                    };
                    device
                        .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                        .expect("failed to create descriptor set layout")
                })
        } else {
            // no associated typeid, create an "anonymous" descriptor set layout
            let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
                binding_count: bindings.len() as u32,
                p_bindings: bindings.as_ptr(),
                ..Default::default()
            };
            device
                .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                .expect("failed to create descriptor set layout")
        }
    }

    /// Creates a descriptor set layout for the given type.
    pub(crate) unsafe fn get_or_create_descriptor_set_layout_for_type<T: StaticArguments>(
        &self,
    ) -> vk::DescriptorSetLayout {
        self.get_or_create_descriptor_set_layout(Some(T::TYPE_ID), T::LAYOUT)
    }

    /*/// Creates a sampler object from a type implementing `ToSampler`.
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
                    }))
                })
        } else {
            todo!()
        }
    }*/

    /*pub(crate) fn destroy_sampler(&self, id: SamplerId) {
        let mut inner = self.inner.lock().unwrap();
        inner.samplers.destroy_on_frame_completed(inner.current_frame, );
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
    }*/
}

pub unsafe fn create_device_and_context(present_surface: Option<vk::SurfaceKHR>) -> (Device, graal::Context) {
    let (backend_device, backend_context) = graal::create_device_and_context(present_surface);

    (
        Device {
            inner: Arc::new(Mutex::new(DeviceInner {
                current_frame: Default::default(),
                descriptor_set_layout_by_typeid: Default::default(),
                sampler_by_typeid: Default::default(),
            })),
            backend: backend_device,
        },
        backend_context,
    )
}
