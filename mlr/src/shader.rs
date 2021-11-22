//! Shader
use std::any::TypeId;
use std::cell::Cell;
use std::collections::HashSet;
use crate::context::Context;
use graal::{ResourceId, vk};
use graal_spirv::typedesc;
use std::sync::Arc;
use thiserror::Error;
use crate::{Buffer, BufferView};
use crate::descriptor::DescriptorSetLayoutId;
use crate::image::ImageAny;
use crate::sampler::Sampler;


pub enum ArgumentDesc {
    Texture {
        location: usize,
    },
    UniformBuffer {
        location: usize,
        ty: typedesc::TypeDesc<'static>
    },
    Uniform {
        location: usize,
        ty: typedesc::TypeDesc<'static>
    }
}

/// A trait implemented by types that can produce one or more descriptors.
unsafe trait DescriptorSource {
    type DescriptorWriteType;
    const ARRAY_LEN: usize;

    fn to_descriptors(&self) -> Self::DescriptorWriteType;

    /// Returns the number of descriptors in this binding.
    fn descriptor_count(&self) -> u32 {
        1
    }
}

struct TextureDescriptor<'a> {
    image: &'a ImageAny,
    sampler: &'a Sampler,
    // placeholder for the descriptor update template
    descriptor: Cell<vk::DescriptorImageInfo>
}

impl<'a> TextureDescriptor<'a> {
    pub fn new(image: &'a ImageAny, sampler: &'a Sampler) -> TextureDescriptor<'a> {
        TextureDescriptor {
            image,
            sampler,
            descriptor: Default::default()
        }
    }
}

unsafe impl<'a> DescriptorSource for TextureDescriptor<'a> {
    type DescriptorWriteType = vk::DescriptorImageInfo;
    const ARRAY_LEN: usize = 1;
    fn to_descriptors(&self) -> Self::DescriptorWriteType {
        todo!()
    }
}


struct BufferDescriptor<'a> {
    buffer: &'a Buffer,
    // placeholder for the descriptor update template
    descriptor: Cell<vk::DescriptorBufferInfo>,
}

impl<'a> BufferDescriptor<'a> {
    pub fn new(buffer: &'a Buffer) -> BufferDescriptor<'a> {
        BufferDescriptor {
            buffer,
            descriptor: Default::default()
        }
    }
}

unsafe impl<'a> DescriptorSource for BufferDescriptor<'a> {
    type DescriptorWriteType = vk::DescriptorBufferInfo;
    const ARRAY_LEN: usize = 1;
    fn to_descriptors(&self) -> Self::DescriptorWriteType {
        todo!()
    }
}

/// Implementation of DescriptorSource for arrays.
unsafe impl<T: DescriptorSource, const N: usize> DescriptorSource for [T; N] {
    type DescriptorWriteType = [T::DescriptorWriteType; N];
    const ARRAY_LEN: usize = N;
    fn to_descriptors(&self) -> Self::DescriptorWriteType {
        todo!()
    }
}

pub trait ShaderArguments {
    const DESCRIPTOR_SET_LAYOUT_BINDINGS: &'static [vk::DescriptorSetLayoutBinding];
    /// TODO doc
    fn to_argument_block(&self) -> ArgumentBlock;
    /// TODO doc
    fn type_id() -> TypeId;
}

//Note: `Clone` would require `WithSpan: Clone`.
#[derive(Debug, Error)]
pub enum CreateShaderError {
    #[error(transparent)]
    Vulkan(#[from] vk::Result),
}

impl From<vk::Result> for CreateShaderError {
    fn from(code: vk::Result) -> Self {
        CreateShaderError::Vulkan(code)
    }
}

/// Shaders
pub struct Shader {
    static_spirv: Option<&'static [u32]>,
    device: Option<Arc<graal::Device>>,
    shader_module: vk::ShaderModule,
}

impl Shader {
    /// Creates a new shader.
    ///
    /// The compilation of the shader module is deferred to its first use in a `Context`.
    pub fn from_spirv_static(spirv: &'static [u32]) -> Shader {
        Shader {
            static_spirv: Some(spirv),
            device: None,
            shader_module: Default::default(),
        }
    }

    /// Creates a new shader immediately.
    pub fn from_spirv(context: &Context, spirv: &[u32]) -> Result<Shader, CreateShaderError> {
        let device = context.device().clone();
        let vk_device = &device.device;

        let shader_module = unsafe {
            vk_device.create_shader_module(
                &vk::ShaderModuleCreateInfo {
                    flags: Default::default(),
                    code_size: code.len() * 4,
                    p_code: code.as_ptr(),
                    ..Default::default()
                },
                None,
            )?
        };

        Ok(Shader {
            static_spirv: None,
            device: Some(device),
            shader_module,
        })
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            // TODO safety
            self.device
                .device
                .destroy_shader_module(self.shader_module, None)
        }
    }
}

struct PassImageDependency {
    id: graal::ImageId,
    access_mask: graal::vk::AccessFlags,
    stage_mask: graal::vk::PipelineStageFlags,
    initial_layout: graal::vk::ImageLayout,
    final_layout: graal::vk::ImageLayout,
}

struct PassBufferDependency {
    id: graal::BufferId,
    access_mask: graal::vk::AccessFlags,
    stage_mask: graal::vk::PipelineStageFlags,
}

pub(crate) struct PassDependencies {
    images: Vec<PassImageDependency>,
    buffers: Vec<PassBufferDependency>,
    groups: Vec<graal::ResourceGroupId>,
}

impl PassDependencies {
    pub(crate) fn new() -> PassDependencies {
        PassDependencies {
            images: vec![],
            buffers: vec![],
            groups: vec![]
        }
    }
}

/// Contains all data to create a descriptor set during a pass
struct DescriptorSetInit {
    layout: DescriptorSetLayoutId,
    descriptor_data_offset: usize,
}

pub struct ArgumentBlock<'a> {
    layout: DescriptorSetLayoutId,
    dependencies: PassDependencies,
    descriptor_data_block: Vec<u8>
}

impl<'a> ArgumentBlock<'a> {
    pub fn get_resource_references(&self) {
        //
    }

    pub fn write_
}

enum Command {
    BindDescriptorSet {
        set: usize,
        init: DescriptorSetInit
    }
}

const MAX_DESCRIPTOR_SETS: usize = 8;

pub(crate) struct Batch {
    // can descriptor sets be shared between passes? no
    // once a batch is flushed, all descriptor sets are discarded
    descriptor_data: Vec<u8>,
    commands: Vec<Command>,
    batch_index: usize,
    descriptor_sets: [u64; MAX_DESCRIPTOR_SETS]
}

impl Context {

    fn write_descriptor_data(&mut self, block: &ArgumentBlock) -> usize {
        // problem: where is the descriptor data?
    }

    fn bind_argument_block(&mut self, set: usize, block: &ArgumentBlock) {
        //self.ensure_batch_started();



        if self.current_batch.descriptor_sets[set] != block.id {


            batch.commands.push(Command::BindDescriptorSet {
                 set,

            })
        }
    }

    fn flush_pending_draw_calls(&mut self) {
        let descriptor_cache = self.descriptor_cache.clone();
        self.context.pass_commands(move |ctx, cb| {
            let mut pipeline_layout = vk::PipelineLayout::default();
            for command in batch.commands {
                match command {
                    Command::BindDescriptorSet { set, init } => {
                        let descriptor_set = descriptor_cache.create_descriptor_set(block);
                        unsafe {
                            ctx.vulkan_device().cmd_bind_descriptor_sets(
                                cb,
                                vk::PipelineBindPoint::GRAPHICS,
                                pipeline_layout,
                                set,
                                &[descriptor_set],
                                &[],
                            );
                        }
                    }
                }
            }
        });

        self.context.end_pass();
    }


    pub fn draw(&mut self, arg_blocks: &[&ArgumentBlock], vertex_buffers: &[BufferView], index_buffer: Option<BufferView>)
    {
        for (i,ab) in arg_blocks.iter().enumerate() {
            self.bind_argument_block()
        }

        let mut new_resource_groups = HashSet::new();
        let mut new_resources = HashSet::new();

        for ab in arg_blocks {
            ab.collect_resource_refs(&mut new_groups, &mut new_resources);
        }

        for vb in vertex_buffers {
            vb.ensure_created(self);
        }

        if new_groups == self.resource_groups && new_resources == self.resource_groups {
            // same ref'd resources, we merge with the previous draw call
        } else {
            // not the same resources, flush and start a new batch
            self.flush_draw_calls();
        }

        // just submit different passes? graal will just reuse the same command buffer if the refs are the same

        // if one arg block changes,
    }
}


// RenderTask object
// -> 'a ref to resources
// implements RenderTask
// ->

// RenderTask object moved into the pass callback
pub trait RenderTask<'a>
{
    /// Register resource dependencies.
    /// It's important to be precise here.

    fn register_dependencies(&self);

    /// runs the pass
    fn execute(&self, ctx: &mlr::DrawContext);
}

// e.g
struct SceneRenderTask<'a> {
    scene: &'a Scene,
    camera: Camera,
}


fn test() {
    #[derive(mlr::ShaderArguments)]
    #[repr(C)]
    struct SceneArguments {
        // uniform variables will be put in a single uniform buffer, at location 0
        u_view_matrix: Mat4,
        u_proj_matrix: Mat4,
        u_view_proj_matrix: Mat4,
        u_inverse_proj_matrix: Mat4,
    }

    #[derive(mlr::ShaderArguments)]
    #[repr(C)]
    struct MaterialArguments<'a> {
        u_color: Vec4,
        #[argument(sampled_image,binding=1)] t_color: TextureDescriptor<'a>
    }


    // issue: the draw pass doesn't know the resources used inside
    //
    ctx.draw_pass(|| {

        let scene_args = ctx.create_argument_block(SceneArguments {
            u_view_matrix: (),
            u_proj_matrix: (),
            u_view_proj_matrix: (),
            u_inverse_proj_matrix: ()
        });

        for batch in material_batches.iter() {
            let material_args = ctx.create_argument_block(
                MaterialArguments {
                    u_color: (),
                    t_color: TextureDescriptor::new(&batch.texture, Sampler::linear())
                });

            for mesh in batch.objects.iter() {

                // 1000 objects, 4 materials
                // => 4000 ArgumentBlocks
                // => 4000 Arc<[u8]> alive until command buffer generation

                let object_args = ctx.create_argument_block();

                // Q: is there a memory dependency between the args and the previous draw?
                // -> we don't care, just create a pass on every
                ctx.draw(&[&scene_args, &material_args, &object_args])
            }
        }
    });

    // other solution: create argument blocks during command buffer generation

    // problem: draw pass must specify dependencies
    // problem: command generation callback must borrow stuff (scene?)

        static_resource_group,

        || {
        // command generation callback


    });

}