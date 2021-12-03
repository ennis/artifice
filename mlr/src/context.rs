use crate::{
    arguments::{ArgumentBlock, Arguments},
    descriptor::{DescriptorSetAllocator, DescriptorSetLayoutId},
};
use graal::{ash, swapchain::Swapchain, vk, Device, FrameNumber};
use mlr::{buffer::BufferData, sampler::SamplerType};
use slotmap::SlotMap;
use std::{
    alloc::Layout,
    any::TypeId,
    collections::{HashMap, VecDeque},
    hash::Hash,
    mem, ptr,
    ptr::slice_from_raw_parts,
    slice,
    sync::Arc,
};

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

/*/// Hashable wrapper over `vk::SamplerCreateInfo`.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub(crate) struct WrapHash<T: Copy+'static>(pub(crate) T);

impl<T> Hash for WrapHash<T> where WrapHash<T>: bytemuck::Pod {
    fn hash<H: Hasher>(&self, state: &mut H) {
        todo!()
    }
}*/

pub(crate) struct EvalContext {
    descriptor_allocators: SlotMap<DescriptorSetLayoutId, DescriptorSetAllocator>,
    descriptor_set_layout_by_typeid: HashMap<TypeId, DescriptorSetLayoutId>,
    sampler_by_typeid: HashMap<TypeId, vk::Sampler>,
    in_flight: VecDeque<InFlightFrameResources>,
    frame_resources: InFlightFrameResources,
}

/// Context passed to the pass setup closure.
pub struct PassBuilder<'a, 'b> {
    pub(crate) frame: &'a mut graal::Frame<'b, EvalContext>,
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

}*/

pub struct ContextResources {
    device: Arc<graal::Device>,
    descriptor_allocators: SlotMap<DescriptorSetLayoutId, DescriptorSetAllocator>,
    descriptor_set_layout_by_typeid: HashMap<TypeId, DescriptorSetLayoutId>,
    sampler_by_typeid: HashMap<TypeId, vk::Sampler>,
    in_flight: VecDeque<InFlightFrameResources>,
    frame_resources: InFlightFrameResources,
}

impl ContextResources {
    fn new(device: Arc<Device>) -> ContextResources {
        ContextResources {
            device,
            descriptor_allocators: SlotMap::with_key(),
            descriptor_set_layout_by_typeid: Default::default(),
            sampler_by_typeid: Default::default(),
            in_flight: Default::default(),
            frame_resources: Default::default(),
        }
    }

    pub fn vulkan_device(&self) -> &graal::ash::Device {
        &self.device.device
    }

    /// Creates a transient image view that will be deleted at the end of the frame.
    ///
    /// # Safety
    ///
    /// TODO see the safety requirements of `vkCreateImageView`?
    pub unsafe fn create_transient_image_view(
        &mut self,
        create_info: &vk::ImageViewCreateInfo,
    ) -> vk::ImageView {
        let device = &self.device.device;
        let image_view = device.create_image_view(create_info, None).unwrap();
        self.frame_resources.image_views.push(image_view);
        image_view
    }

    /// Creates a sampler object.
    ///
    /// The returned object lives as long as the context is alive.
    pub(crate) unsafe fn get_or_create_sampler(
        &mut self,
        sampler: impl SamplerType,
    ) -> vk::Sampler {
        let device = &self.device.device;
        if let Some(type_id) = sampler.unique_type_id() {
            *self
                .sampler_by_typeid
                .entry(type_id)
                .or_insert_with(|| sampler.to_sampler(device))
        } else {
            todo!()
        }
    }

    /// Creates a descriptor set layout and an associated allocator.
    pub(crate) fn get_or_create_descriptor_set_layout(
        &mut self,
        type_id: Option<TypeId>,
        bindings: &[vk::DescriptorSetLayoutBinding],
        update_template_entries: Option<&[vk::DescriptorUpdateTemplateEntry]>,
    ) -> (vk::DescriptorSetLayout, DescriptorSetLayoutId) {
        let device = &self.device.device;
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
            allocators.insert(DescriptorSetAllocator::new(
                device,
                bindings,
                update_template_entries,
            ))
        };

        (self.descriptor_allocators.get(id).unwrap().layout, id)
    }

    /*/// Returns the descriptor set allocator for the given layout id.
    pub(crate) fn get_descriptor_set_allocator(
        &mut self,
        id: DescriptorSetLayoutId,
    ) -> &mut DescriptorSetAllocator {
        self.descriptor_allocators.get_mut(id).unwrap()
    }*/

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

    pub(crate) fn upload_slice<T: BufferData>(
        &mut self,
        data: &[T],
        usage: vk::BufferUsageFlags,
    ) -> (vk::Buffer, vk::DeviceSize) {
        let buffer = self.allocate_upload_buffer(mem::size_of_val(data), usage);
        // copy data
        unsafe { ptr::copy_nonoverlapping(data.as_ptr(), buffer.ptr as *mut T, data.len()) }
        (buffer.handle, buffer.offset)
    }

    /// Creates an argument block.
    pub(crate) fn create_argument_block<T: Arguments>(&mut self, mut args: T) -> ArgumentBlock {
        let (_, layout_id) = self.get_or_create_descriptor_set_layout(
            args.unique_type_id(),
            args.get_descriptor_set_layout_bindings(),
            args.get_descriptor_set_update_template_entries(),
        );
        let allocator = self.descriptor_allocators.get_mut(layout_id).unwrap();
        let update_template = allocator.update_template();
        let descriptor_set = allocator.allocate(&self.device.device);
        self.frame_resources
            .descriptor_sets
            .push((layout_id, descriptor_set));

        // SAFETY: TODO?
        unsafe {
            args.update_descriptor_set(self, descriptor_set, update_template);
        }

        ArgumentBlock { descriptor_set }
    }
}

/// MLR context.
pub struct Context {
    pub(crate) backend: graal::Context,
    pub(crate) resources: ContextResources,
}

impl Context {
    /// Creates a new context.
    pub fn new(device: graal::Device) -> Context {
        let backend = graal::Context::with_device(device);
        let device = backend.device().clone();
        Context {
            backend,
            resources: ContextResources::new(device),
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

    /*/// Starts a frame.
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
    }*/
}
