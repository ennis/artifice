use crate::{
    buffer::{BufferAny, BufferData},
    context::Context,
    image::ImageAny,
    ArgumentBlock, Arguments,
};
use graal::{vk, Device, FrameCreateInfo, ImageId, ResourceGroupId, ResourceId};
use mlr::ResourceVisitor;
use std::{
    alloc::Layout,
    cell::Cell,
    collections::{HashMap, HashSet},
    io::Write,
    mem, ptr,
    sync::Arc,
};

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

/// Commands
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

// issue: transient buffers
// -> frame.upload(): should return buffer with lifetime bound to the frame object => cannot mut borrow
// -> frame.start_graphics_pass(): mut borrows
//
// Do the same thing as the Context/Device split:
// Split frame into FrameResources / FrameBuilder
//
// context.start_frame(&'a mut self) -> (Frame<'a>, FrameResources<'a>)
// frame.
// context.finish_frame(&
//
// or simply use device.create_buffer() with a hint?
// Problem: would like to allocate some host visible memory and get a pointer to it:
// what's the lifetime of the pointer?
//
// wgpu: borrows buffer object, but no

// -> frame has an internal upload buffer for argument blocks, but it is otherwise inaccessible
// to users. Instead, require the user to pass their own upload buffers. This way they can size
// the chunks as required.

pub struct RenderPass<'a, 'b> {
    //frame: &'b mut Frame<'a>,
    pass: graal::PassBuilder<'b, 'a, Context>,
    commands: Vec<Command>,
}

impl<'a, 'b> ResourceVisitor for RenderPass<'a, 'b> {
    fn visit_image(
        &mut self,
        image: &ImageAny,
        access_mask: vk::AccessFlags,
        stage_mask: vk::PipelineStageFlags,
        layout: vk::ImageLayout,
    ) -> bool {
        self.pass
            .add_image_dependency(image.id(), access_mask, stage_mask, layout, layout);
        true
    }

    fn visit_buffer(
        &mut self,
        buffer: &BufferAny,
        access_mask: vk::AccessFlags,
        stage_mask: vk::PipelineStageFlags,
    ) -> bool {
        self.pass
            .add_buffer_dependency(buffer.id(), access_mask, stage_mask);
        true
    }
}

impl<'a, 'b> RenderPass<'a, 'b> {
    /// vkCmdBindDescriptorSet
    pub fn bind_argument_block<T: Arguments>(&mut self, number: u32, args: &ArgumentBlock<T>) {
        args.args.walk_resources(self);
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

    pub fn finish(mut self) {
        let commands = self.commands;

        self.pass
            .set_record_callback(move |record, ctx, command_buffer| {
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
        pass.finish();
    }
}

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
struct FrameResources {
    frame_number: graal::FrameNumber,
    descriptor_sets: Vec<(graal::DescriptorSetLayoutId, vk::DescriptorSet)>,
    image_views: Vec<vk::ImageView>,
    upload_chunks: Vec<UploadChunk>,
    current_upload_chunk: Option<UploadChunk>,
}

impl Default for FrameResources {
    fn default() -> Self {
        FrameResources {
            frame_number: Default::default(),
            descriptor_sets: vec![],
            //framebuffers: vec![],
            image_views: vec![],
            upload_chunks: vec![],
            current_upload_chunk: None,
        }
    }
}

impl FrameResources {}

pub struct Frame<'a> {
    pub(crate) context: &'a mut Context,
    pub(crate) backend: graal::Frame<'a, Context>,
    pub(crate) resources: FrameResources,
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
        let device = self.context.vulkan_device();
        let image_view = device.create_image_view(create_info, None).unwrap();
        self.resources.image_views.push(image_view);
        image_view
    }

    /// Allocates a descriptor set.
    pub(crate) fn allocate_descriptor_set_for_arguments<T: Arguments>(
        &mut self,
        device: &graal::ash::Device,
        args: T,
    ) -> (graal::DescriptorSetLayoutId, vk::DescriptorSet) {
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

impl<'a> Frame<'a> {
    /// Starts a render pass.
    pub fn start_render_pass<'b>(&'b mut self, desc: &RenderPassDescriptor) -> RenderPass<'a, 'b> {
        RenderPass {
            pass: self.backend.start_graphics_pass("render_pass"),
            commands: vec![],
        }
    }

    /// Finishes this frame
    pub fn finish(mut self) {
        self.resources.frame_number = self.backend.frame_number();

        let _frame_future = self.backend.finish(
            self.context,
            /*|context, device, frame_number| {
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
            }*/
        );

        // destroy transient resources (the backend handles the actual deferred deletion)
        let device = self.context.backend.device();
        for image_view in self.resources.image_views {
            unsafe { device.destroy_image_view(image_view) }
        }
    }
}
