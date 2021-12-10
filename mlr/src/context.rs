use crate::{
    arguments::{ArgumentBlock, Arguments, StaticArguments},
};
use graal::{ash, FrameNumber, swapchain::Swapchain, vk};
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
use graal::descriptor::{DescriptorSetAllocator, DescriptorSetLayoutId};

unsafe fn place_aligned(layout: &Layout, ptr: &mut *mut u8, space: &mut usize) -> *mut u8 {
    let ptr_usize = *ptr as usize;
    let mut off = ptr_usize & (layout.align() - 1);
    if off > 0 {
        off = layout.align() - off;
    }
    if ptr_usize + off + layout.size() > *space {
        ptr::null_mut()
    } else {
        *space -= off + layout.size();
        *ptr = ptr.add(off);
        *ptr
    }
}

#[derive(Copy, Clone, Debug)]
struct MappedBufferSliceInfo {
    ptr: *mut u8,
    handle: vk::Buffer,
    offset: vk::DeviceSize,
    size: vk::DeviceSize,
}

struct UploadChunk {
    buffer: graal::BufferInfo,
    base: *mut u8,
    ptr: *mut u8,
    space: usize,
}

impl UploadChunk {
    fn new(device: &graal::Device, usage: vk::BufferUsageFlags, byte_size: usize) -> UploadChunk {
        let create_info = graal::BufferResourceCreateInfo {
            usage,
            byte_size: byte_size as u64,
            map_on_create: true,
        };

        let buffer = device.create_buffer(
            "upload buffer",
            graal::MemoryLocation::CpuToGpu,
            &create_info,
        );
        let ptr = buffer
            .mapped_ptr
            .expect("buffer was not mapped in memory")
            .as_ptr() as *mut u8;

        UploadChunk {
            buffer,
            base: ptr,
            ptr,
            space: byte_size,
        }
    }

    unsafe fn allocate(&mut self, layout: &Layout) -> Option<MappedBufferSliceInfo> {
        let ptr = place_aligned(layout, &mut self.ptr, &mut self.space);
        if !ptr.is_null() {
            Some(MappedBufferSliceInfo {
                ptr,
                handle: self.buffer.handle,
                offset: ptr.offset_from(self.base) as vk::DeviceSize,
                size: layout.size() as vk::DeviceSize,
            })
        } else {
            None
        }
    }
}

const UPLOAD_CHUNK_SIZE: usize = 4 * 1024 * 1024;
const UPLOAD_DEDICATED_THRESHOLD_SIZE: usize = 1024 * 1024;

/// Transient objects that should be deleted or recycled once the frame has completed execution.
struct InFlightFrameResources {
    frame_number: FrameNumber,
    descriptor_sets: Vec<(DescriptorSetLayoutId, vk::DescriptorSet)>,
    framebuffers: Vec<vk::Framebuffer>,
    image_views: Vec<vk::ImageView>,
    upload_chunks: Vec<UploadChunk>,
    current_upload_chunk: Option<UploadChunk>,
}

impl Default for InFlightFrameResources {
    fn default() -> Self {
        InFlightFrameResources {
            frame_number: Default::default(),
            descriptor_sets: vec![],
            framebuffers: vec![],
            image_views: vec![],
            upload_chunks: vec![],
            current_upload_chunk: None,
        }
    }
}

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


#[derive(Copy, Clone, Debug)]
pub enum AttachmentLoadOp<V> {
    Load,
    Clear { value: V },
    DontCare,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum AttachmentStoreOp {
    Store,
    DontCare,
}

/// Basically taken from wgpu.
#[derive(Copy, Clone, Debug)]
pub struct RenderPassColorAttachment<'a> {
    /// TODO subresources
    pub attachment: &'a ImageAny,
    pub load_op: AttachmentLoadOp<[f64; 4]>,
    pub store_op: AttachmentStoreOp,
}

#[derive(Copy, Clone, Debug)]
pub struct RenderPassDepthStencilAttachment<'a> {
    /// TODO subresources
    pub attachment: &'a ImageAny,
    /// TODO separate depth/stencil ops
    pub load_op: AttachmentLoadOp<[f64; 4]>,
    pub store_op: AttachmentStoreOp,
}

/// Basically taken from wgpu.
#[derive(Copy, Clone, Debug)]
pub struct RenderPassDescriptor<'a, 'b> {
    pub color_attachments: &'b [RenderPassColorAttachment<'a>],
    pub depth_stencil_attachment: Option<RenderPassDepthStencilAttachment<'a>>,
}

impl<'a> Frame<'a> {
    /// Creates a transient image view that will be deleted at the end of the frame.
    ///
    /// # Safety
    ///
    /// TODO see the safety requirements of `vkCreateImageView`?
    pub unsafe fn create_transient_image_view(
        &mut self,
        create_info: &vk::ImageViewCreateInfo,
    ) -> vk::ImageView {
        let device = &self.context.vulkan_device();
        let image_view = device.create_image_view(create_info, None).unwrap();
        self.resources.image_views.push(image_view);
        image_view
    }


    /// Allocates a descriptor set.
    pub(crate) fn allocate_descriptor_set_for_arguments<T: Arguments>(
        &mut self,
        device: &graal::ash::Device,
        args: T,
    ) -> (DescriptorSetLayoutId, vk::DescriptorSet) {
        let (_, layout_id) = self.get_or_create_descriptor_set_layout(
            args.unique_type_id(),
            args.get_descriptor_set_layout_bindings(),
            args.get_descriptor_set_update_template_entries(),
        );
        let allocator = self.descriptor_allocators.get_mut(layout_id).unwrap();
        let descriptor_set = allocator.allocate(device);
        (layout_id, descriptor_set)
    }

    /// Allocate space in the upload buffer.
    fn allocate_upload_buffer(
        &mut self,
        byte_size: usize,
        usage: vk::BufferUsageFlags,
    ) -> MappedBufferSliceInfo {
        let device = self.device.as_ref();
        if byte_size > UPLOAD_DEDICATED_THRESHOLD_SIZE {
            // allocate a dedicated chunk
            let chunk = UploadChunk::new(device, usage, byte_size);
            let slice = MappedBufferSliceInfo {
                ptr: chunk.base,
                handle: chunk.buffer.handle,
                offset: 0,
                size: byte_size as vk::DeviceSize,
            };
            self.frame_resources.upload_chunks.push(chunk);
            slice
        } else {
            let mut tried_once = false;
            loop {
                let chunk = self
                    .frame_resources
                    .current_upload_chunk
                    .get_or_insert_with(|| {
                        UploadChunk::new(
                            device,
                            vk::BufferUsageFlags::INDEX_BUFFER
                                | vk::BufferUsageFlags::TRANSFER_SRC
                                | vk::BufferUsageFlags::INDIRECT_BUFFER
                                | vk::BufferUsageFlags::VERTEX_BUFFER
                                | vk::BufferUsageFlags::UNIFORM_BUFFER,
                            UPLOAD_CHUNK_SIZE,
                        )
                    });

                // FIXME: what about other kinds of buffers (vertex buffers?)
                let align = device
                    .physical_device_properties()
                    .limits
                    .min_uniform_buffer_offset_alignment as usize;
                // SAFETY: TODO
                let result =
                    unsafe { chunk.allocate(&Layout::from_size_align(byte_size, align).unwrap()) };
                if let Some(result) = result {
                    return result;
                } else {
                    // not enough space in current chunk, retire current buffer and try again
                    self.frame_resources
                        .upload_chunks
                        .push(self.frame_resources.current_upload_chunk.take().unwrap());
                    if !tried_once {
                        tried_once = true;
                        continue;
                    } else {
                        panic!("failed to allocate upload buffer")
                    }
                }
            }
        }
    }

    pub fn upload_slice<T: BufferData>(
        &mut self,
        data: &[T],
        usage: vk::BufferUsageFlags,
    ) -> (vk::Buffer, vk::DeviceSize) {
        let buffer = self.allocate_upload_buffer(mem::size_of_val(data), usage);
        // copy data
        unsafe { ptr::copy_nonoverlapping(data.as_ptr(), buffer.ptr as *mut T, data.len()) }
        (buffer.handle, buffer.offset)
    }

}

pub struct Frame<'a> {
    context: &'a mut Context,
    backend: graal::Frame<'a, Context>,
    resources: InFlightFrameResources,
}

impl<'a> Frame<'a> {
    /// Starts a render pass.
    pub fn start_render_pass<'b>(&'b mut self, desc: &RenderPassDescriptor) -> RenderPass<'a, 'b> {
        RenderPass {
            frame: self,
            commands: vec![],
        }
    }

    /// Finishes this frame
    pub fn finish(mut self) {
        self.resources.frame_number = self.backend.frame_number();

        let _frame_future = self.context.backend.finish_frame(
            self.backend,
            self.context,
            |context, device, frame_number| {
                while let Some(in_flight_frame) = context.in_flight.pop_front() {
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

pub enum Command {
    BindDescriptorSet {
        number: u32,
        descriptor_set: vk::DescriptorSet,
    },
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },
}

pub struct RenderPass<'a, 'b> {
    frame: &'b mut Frame<'a>,
    commands: Vec<Command>,
}

impl<'a, 'b> RenderPass<'a, 'b> {

    /// vkCmdBindDescriptorSet
    pub fn bind_argument_block<T: Arguments>(&mut self, number: u32, args: ArgumentBlock<T>) {
        // ideally, I'd like to start recording the buffer now
        self.commands.push(Command::BindDescriptorSet {
            number,
            descriptor_set: args.descriptor_set.get(),
        });
    }

    /// vkCmdDraw
    pub fn draw(
        &mut self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        self.commands.push(Command::Draw {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        })
    }

    pub fn finish(self) {
        let commands = self.commands;

        self.frame
            .backend
            .pass_set_record_callback(move |record, ctx, command_buffer| {
                let device = record.context.vulkan_device();

                let mut current_pipeline_layout = vk::PipelineLayout::default();

                // translate commands
                unsafe {
                    for cmd in commands {
                        match cmd {
                            Command::BindDescriptorSet {
                                number,
                                descriptor_set,
                            } => {
                                // TODO dynamic offsets
                                device.cmd_bind_descriptor_sets(
                                    command_buffer,
                                    vk::PipelineBindPoint::GRAPHICS,
                                    current_pipeline_layout,
                                    number,
                                    &[descriptor_set],
                                    &[],
                                );
                            }
                            Command::Draw { .. } => {}
                        }
                    }
                }
            });

        self.frame.backend.end_pass();
    }
}

/// MLR context.
pub struct Context {
    pub(crate) backend: graal::Context,
    pub(crate) in_flight: VecDeque<InFlightFrameResources>,
}

impl Context {

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
            resources: Default::default()
        }
    }
}
