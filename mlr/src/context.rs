use crate::{
    descriptor::{DescriptorSetAllocator, DescriptorSetLayoutId},
    shader::ArgumentBlock,
    ShaderArguments,
};
use graal::{vk, Device, FrameNumber};
use slotmap::SlotMap;
use std::{
    any::TypeId,
    collections::{HashMap, VecDeque},
    mem,
    sync::Arc,
};

/// Transient objects that should be deleted or recycled once the frame has completed execution.
struct InFlightFrameResources {
    frame_number: FrameNumber,
    descriptor_sets: Vec<(DescriptorSetLayoutId, vk::DescriptorSet)>,
    framebuffers: Vec<vk::Framebuffer>,
    image_views: Vec<vk::ImageView>,
}

impl Default for InFlightFrameResources {
    fn default() -> Self {
        InFlightFrameResources {
            frame_number: Default::default(),
            descriptor_sets: vec![],
            framebuffers: vec![],
            image_views: vec![],
        }
    }
}

pub(crate) struct EvalContext {
    descriptor_allocators: SlotMap<DescriptorSetLayoutId, DescriptorSetAllocator>,
    descriptor_set_layout_by_typeid: HashMap<TypeId, DescriptorSetLayoutId>,
    in_flight: VecDeque<InFlightFrameResources>,
    frame_resources: InFlightFrameResources,
}

/// Context passed to the pass setup closure.
pub struct PassBuilder<'a, 'b> {
    pub(crate) frame: &'a mut graal::Frame<'b, EvalContext>,
}

/// Context for recording commands during pass evaluation.
pub struct RecordingContext<'a, 'b> {
    pub(crate) backend: &'a mut graal::RecordingContext<'b>,
    pub(crate) eval_ctx: &'a mut EvalContext,
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
                };
                record_fn(&mut eval_ctx);
            })
    }

    /// Finishes this frame.
    pub fn finish(mut self) {
        self.context.eval_ctx.frame_resources.frame_number = self.backend.frame_number();

        let _frame_future = self.context.backend.finish_frame(
            self.backend,
            &mut self.context.eval_ctx,
            |eval_context, device, frame_number| {
                while let Some(in_flight_frame) = eval_context.in_flight.pop_front() {
                    if in_flight_frame.frame_number <= frame_number {
                        // this frame has finished: destroy or recycle all objects not in use anymore
                        unsafe {
                            for fb in in_flight_frame.framebuffers {
                                device.device.destroy_framebuffer(fb, None);
                            }
                            for iv in in_flight_frame.image_views {
                                device.device.destroy_image_view(iv, None);
                            }
                            for (layout, ds) in in_flight_frame.descriptor_sets {
                                let allocator = eval_context.get_descriptor_set_allocator(layout);
                                allocator.free(ds);
                            }
                        }
                    } else {
                        break;
                    }
                }
            },
        );

        let in_flight_frame_resources = mem::take(&mut self.context.eval_ctx.frame_resources);
        self.context
            .eval_ctx
            .in_flight
            .push_back(in_flight_frame_resources);
    }
}

impl EvalContext {
    /// Creates a descriptor set layout and an associated allocator.
    pub(crate) fn get_or_create_descriptor_set_layout(
        &mut self,
        device: &graal::ash::Device,
        type_id: Option<TypeId>,
        bindings: &[vk::DescriptorSetLayoutBinding],
        update_template_entries: Option<&[vk::DescriptorUpdateTemplateEntry]>,
    ) -> (vk::DescriptorSetLayout, DescriptorSetLayoutId) {
        let mut allocators = &mut self.descriptor_allocators;
        let id = if let Some(type_id) = type_id {
            *self
                .descriptor_set_layout_by_typeid
                .entry(type_id)
                .or_insert_with(|| {
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

        (self.descriptor_allocators.get(id).unwrap().layout, id)
    }

    /// Returns the descriptor set allocator for the given layout id.
    pub(crate) fn get_descriptor_set_allocator(
        &mut self,
        id: DescriptorSetLayoutId,
    ) -> &mut DescriptorSetAllocator {
        self.descriptor_allocators.get_mut(id).unwrap()
    }

    /// Allocates a descriptor set.
    pub(crate) fn allocate_descriptor_set_for_arguments<T: ShaderArguments>(
        &mut self,
        device: &graal::ash::Device,
        args: T,
    ) -> (DescriptorSetLayoutId, vk::DescriptorSet) {
        let (_, layout_id) = self.get_or_create_descriptor_set_layout(
            device,
            args.unique_type_id(),
            args.get_descriptor_set_layout_bindings(),
            args.get_descriptor_set_update_template_entries(),
        );
        let allocator = self.descriptor_allocators.get_mut(layout_id).unwrap();
        let descriptor_set = allocator.allocate(device);
        (layout_id, descriptor_set)
    }
}

/// MLR context.
pub struct Context {
    pub(crate) backend: graal::Context,
    pub(crate) eval_ctx: EvalContext,
}

impl Context {
    /// Creates a new context.
    pub fn new(device: graal::Device) -> Context {
        let backend = graal::Context::with_device(device);
        Context {
            backend,
            eval_ctx: EvalContext {
                descriptor_allocators: SlotMap::with_key(),
                descriptor_set_layout_by_typeid: Default::default(),
                in_flight: VecDeque::new(),
                frame_resources: InFlightFrameResources {
                    frame_number: Default::default(),
                    descriptor_sets: vec![],
                    framebuffers: vec![],
                    image_views: vec![],
                },
            },
        }
    }

    /// Returns a reference to the underlying `graal::Device`
    pub fn device(&self) -> &Arc<graal::Device> {
        self.backend.device()
    }

    /// Returns a reference to the underlying `VkDevice`
    pub fn vulkan_device(&self) -> &graal::ash::Device {
        &self.backend.device().device
    }

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
        }
    }
}
