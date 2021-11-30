use crate::{
    buffer::BufferAny,
    command::Command,
    context::Context,
    descriptor::DescriptorSetLayoutCache,
    image::ImageAny,
    shader::{ArgumentBlock, ArgumentBlockId, ResourceHolder, ResourceVisitor, ShaderArguments},
};
use graal::{
    ash::vk::{DescriptorType, ShaderStageFlags},
    vk, Device, FrameCreateInfo, ImageId, ResourceGroupId, ResourceId,
};
use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
    io::Write,
    sync::Arc,
};

pub type Arena = bumpalo::Bump;

const MAX_TRACKED_DESCRIPTOR_SETS: usize = 24;

pub unsafe trait DescriptorBinding {
    /// Descriptor type.
    const DESCRIPTOR_TYPE: vk::DescriptorType;
    /// Number of descriptors represented in this object.
    const DESCRIPTOR_COUNT: usize;
    /// Which shader stages can access a resource for this binding.
    const SHADER_STAGES: vk::ShaderStageFlags;
    /// Offset to the descriptor update data within this object.
    const UPDATE_OFFSET: usize;

    /// Prepares the descriptor update data.
    // Implementations can access the submission context to upload uniform data, create image views, create samplers, etc.
    fn prepare_update(&self, ctx: &mut PassSubmitCtx);
}

/*#[doc(hidden)]
pub const fn descriptor_update_data_stride<T: DescriptorBinding>() -> usize {
    match T::DESCRIPTOR_TYPE {
        vk::DescriptorType::SAMPLER
        | vk::DescriptorType::COMBINED_IMAGE_SAMPLER
        | vk::DescriptorType::SAMPLED_IMAGE
        | vk::DescriptorType::STORAGE_IMAGE
        | vk::DescriptorType::INPUT_ATTACHMENT => std::mem::size_of::<vk::DescriptorImageInfo>(),
        vk::DescriptorType::UNIFORM_TEXEL_BUFFER
        | vk::DescriptorType::STORAGE_TEXEL_BUFFER
        | vk::DescriptorType::UNIFORM_BUFFER
        | vk::DescriptorType::STORAGE_BUFFER
        | vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC
        | vk::DescriptorType::STORAGE_BUFFER_DYNAMIC => {
            std::mem::size_of::<vk::DescriptorBufferInfo>()
        }
        _ => panic!("unsupported descriptor type")
    }
}

#[doc(hidden)]
pub const fn descriptor_update_data_array_size<T: DescriptorBinding>() -> usize {
    T::DESCRIPTOR_COUNT * descriptor_update_data_stride::<T>()
}*/

/// Sampled image accessor.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
#[derive(mlr::StructLayout)]
pub struct SampledImage {
    image: ImageId,
    descriptor: Cell<vk::DescriptorImageInfo>,
}

unsafe impl DescriptorBinding for SampledImage {
    const DESCRIPTOR_TYPE: vk::DescriptorType = vk::DescriptorType::SAMPLED_IMAGE;
    const SHADER_STAGES: ShaderStageFlags = vk::ShaderStageFlags::ALL;
    const DESCRIPTOR_COUNT: usize = 1;
    const UPDATE_OFFSET: usize = SampledImage::LAYOUT.descriptor.offset;

    fn prepare_update(&self, ctx: &mut PassSubmitCtx) {
        self.descriptor.set(vk::DescriptorImageInfo {
            sampler: Default::default(),
            image_view: Default::default(),
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        })
    }
}

impl SampledImage {
    /// Creates a new `SampledImage` accessor for use in the current pass.
    pub fn new(builder: &mut PassBuilder, image: &ImageAny) -> SampledImage {
        // TODO encode precise stages in const generics.
        let stages = vk::PipelineStageFlags::VERTEX_SHADER
            | vk::PipelineStageFlags::FRAGMENT_SHADER
            | vk::PipelineStageFlags::GEOMETRY_SHADER
            | vk::PipelineStageFlags::TESSELLATION_CONTROL_SHADER
            | vk::PipelineStageFlags::TESSELLATION_EVALUATION_SHADER
            | vk::PipelineStageFlags::COMPUTE_SHADER;
        builder.frame.pass_image_dependency(
            image.id(),
            vk::AccessFlags::SHADER_READ,
            stages,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        SampledImage {
            image: image.id(),
            descriptor: Cell::new(Default::default()),
        }
    }
}

/// Color attachment accessor.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ColorAttachment {
    image: ImageId,
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
        ColorAttachment { image: image.id() }
    }
}

/// Context passed to the pass setup closure.
pub struct PassBuilder<'a, 'b> {
    frame: &'a mut graal::Frame<'b>,
}

///
pub struct PassSubmitCtx<'a, 'b> {
    command_ctx: &'a mut graal::RecordingContext<'b>,
}

impl<'a, 'b> PassSubmitCtx<'a, 'b> {

    /// Creates a transient image view that will be deleted at the end of the frame.
    pub fn create_image_view(&mut self, ) {

    }

    pub fn draw(&mut self, arg_blocks: &[&ArgumentBlock<dyn ShaderArguments>]) {
        //
        for &arg_block in arg_blocks.iter() {}
    }
}

pub struct Frame<'a, 'b> {
    arena: &'a Arena,
    descriptors: &'a mut DescriptorSetLayoutCache,
    frame: graal::Frame<'b>,
}

impl<'a, 'b> Frame<'a, 'b> {
    pub fn device(&self) -> &Arc<Device> {
        &self.frame.device()
    }

    pub fn submit_pass<Setup, Record>(&mut self, name: &str, setup: Setup)
    where
        Setup: FnOnce(&mut PassBuilder) -> Record,
        Record: FnOnce(&mut PassSubmitCtx) + 'b,
    {
        self.frame.start_graphics_pass(name);
        let mut setup_ctx = PassBuilder {
            frame: &mut self.frame,
        };
        let record_fn = setup(&mut setup_ctx);
        self.frame.pass_commands(move |command_ctx| {
            let mut submit_ctx = PassSubmitCtx { command_ctx };
            record_fn(&mut submit_ctx);
        })
    }
}

impl Context {
    pub fn start_frame<'a, 'b>(&'a mut self) -> Frame<'a, 'b> {
        Frame {
            arena: &self.arena,
            descriptors: &mut self.descriptors,
            frame: graal::Frame::new(
                &mut self.context,
                FrameCreateInfo {
                    happens_after: Default::default(),
                    collect_debug_info: false,
                },
            ),
        }
    }
}
