use crate::{
    arguments::{ArgumentBlock, Arguments, StaticArguments},
    frame::Frame,
};
use graal::{ash, swapchain::Swapchain, vk, DescriptorSetLayoutId, FrameNumber};
use mlr::{buffer::BufferData, image::ImageAny, sampler::SamplerType};
use slotmap::{SecondaryMap, SlotMap};
use std::{
    alloc::Layout,
    any::TypeId,
    collections::{HashMap, VecDeque},
    hash::Hash,
    mem,
    ops::{Deref, DerefMut},
    ptr,
    ptr::slice_from_raw_parts,
    slice,
    sync::Arc,
};

/*/// Context for recording commands during pass evaluation.
pub struct RecordingContext<'a, 'b> {
    pub(crate) backend: &'a mut graal::RecordingContext<'b>,
    pub(crate) eval_ctx: &'a mut EvalContext,
    pub(crate) command_buffer: vk::CommandBuffer,
}

impl<'a, 'b> RecordingContext<'a, 'b> {
    /// Returns the underlying vulkan device.
    pub fn vulkan_device(&self) -> &graal::ash::Device {
        self.backend.vulkan_device()
    }

    /// Creates a transient image view that will be deleted at the end of the frame.
    ///
    /// # Safety
    ///
    /// TODO see the safety requirements of `vkCreateImageView`?
    pub unsafe fn create_image_view(
        &mut self,
        create_info: &vk::ImageViewCreateInfo,
    ) -> vk::ImageView {
        let device = self.backend.context.vulkan_device();
        let image_view = device.create_image_view(create_info, None).unwrap();
        self.eval_ctx.frame_resources.image_views.push(image_view);
        image_view
    }

    /// Creates a sampler object from a `SamplerType` instance.
    pub(crate) unsafe fn get_or_create_sampler(
        &mut self,
        sampler: impl SamplerType,
    ) -> vk::Sampler {
        self.eval_ctx
            .get_or_create_sampler(self.backend.context.vulkan_device(), sampler)
    }

    /// Creates an argument block.
    pub(crate) fn create_argument_block<T: ShaderArguments>(
        &mut self,
        mut args: T,
    ) -> ArgumentBlock {
        let device = self.backend.vulkan_device();
        let (_, layout_id) = self.eval_ctx.get_or_create_descriptor_set_layout(
            device,
            args.unique_type_id(),
            args.get_descriptor_set_layout_bindings(),
            args.get_descriptor_set_update_template_entries(),
        );
        let allocator = self.eval_ctx.get_descriptor_set_allocator(layout_id);
        let update_template = allocator.update_template();
        let descriptor_set = allocator.allocate(device);
        self.eval_ctx
            .frame_resources
            .descriptor_sets
            .push((layout_id, descriptor_set));

        // SAFETY: TODO?
        unsafe {
            args.update_descriptor_set(self, descriptor_set, update_template);
        }

        ArgumentBlock { descriptor_set }
    }

    /// Uploads data in a device-visible buffer for this frame.
    ///
    /// Returns buffer + offset.
    pub(crate) fn upload<T: BufferData>(
        &mut self,
        data: &T,
        usage: vk::BufferUsageFlags,
    ) -> (vk::Buffer, vk::DeviceSize) {
        self.eval_ctx
            .upload_slice(self.backend.device(), slice::from_ref(&data), usage)
    }

    /// Uploads data in a device-visible buffer for this frame.
    ///
    /// Returns buffer + offset.
    pub(crate) fn upload_slice<T: BufferData>(
        &mut self,
        data: &[T],
        usage: vk::BufferUsageFlags,
    ) -> (vk::Buffer, vk::DeviceSize) {
        self.eval_ctx
            .upload_slice(self.backend.device(), data, usage)
    }

    /// Draw stuff
    pub fn draw(&mut self, arg_blocks: &[&ArgumentBlock]) {
        for &arg_block in arg_blocks.iter() {}
    }
}

/// A frame.
///
/// TODO better docs.
pub struct Frame<'a> {
    context: &'a mut Context,
    backend: graal::Frame<'a, EvalContext>,
}

impl<'a> Frame<'a> {
    /// Returns the underlying `graal::Device`.
    pub fn device(&self) -> &Arc<Device> {
        &self.backend.device()
    }

    /// Submits a pass
    pub fn submit_pass<Setup, Record>(&mut self, name: &str, setup: Setup)
    where
        Setup: FnOnce(&mut PassBuilder) -> Record,
        Record: FnOnce(&mut RecordingContext) + 'a,
    {
        self.backend.start_graphics_pass(name);
        let mut setup_ctx = PassBuilder {
            frame: &mut self.backend,
        };
        let record_fn = setup(&mut setup_ctx);
        self.backend
            .pass_set_record_callback(move |recording_ctx, eval_ctx, cb| {
                let mut eval_ctx = RecordingContext {
                    backend: recording_ctx,
                    eval_ctx,
                    command_buffer: cb,
                };
                record_fn(&mut eval_ctx);
            });
        self.backend.end_pass();
    }

    pub fn present(&mut self) {}

    /// Finishes this frame.
}

impl EvalContext {
}*/

/// MLR context.
pub struct Context {
    pub(crate) backend: graal::Context,
}

impl Context {
    /// Starts a frame.
    ///
    /// To finish building the frame, call `Frame::finish`.
    pub fn start_frame(&mut self) -> Frame {
        let frame_backend = self.backend.start_frame(graal::FrameCreateInfo {
            happens_after: Default::default(),
            collect_debug_info: false,
        });

        Frame {
            context: self,
            backend: frame_backend,
            resources: Default::default(),
        }
    }
}
