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

pub struct ArgumentBlock<'a> {
    layout: DescriptorSetLayoutId,
    refs: Vec<(ResourceId, graal::vk::PipelineStageFlags, graal::vk::PipelineStageFlags)>
}

impl<'a> ArgumentBlock<'a> {
    pub fn get_resource_references(&self) {
        //
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



impl Context {

    fn flush_draw_calls(&mut self) {

        let descriptor_cache = self.descriptor_cache.clone();

        self.context.add_graphics_pass("flush", move |pass| {
            for img_dep in dependencies.images {
                pass.reference_image(img_dep.id, img_dep.access_mask, img_dep.stage_mask, img_dep.initial_layout, img_dep.final_layout);
            }
            for buf_dep in dependencies.buffers {
                pass.reference_buffer(buf_dep.id, buf_dep.access_mask, buf_dep.stage_mask);
            }
            for group_dep in dependencies.groups {
                pass.reference_group(group_dep);
            }

            pass.set_commands(move |ctx, cb| {

                // what's annoying is that since resource allocation is delayed, it's impossible
                // to create the descriptor sets before the time we build the command buffers.
                // This means that we **invariably** end up with an Arc referencing something
                // in the pass callback: the data to create the descriptor set, and the descriptor set
                // allocator.
                //
                // Is this a problem?
                // -> referencing the descriptor set allocator => there should be one per "command builder" thread anyway
                // -> referencing the data to create the descriptor set => it's just ResourceIds or vk::Buffers

                // what to do with frame resources? descriptor sets, image views, etc.
                // => graal could reclaim them automatically?
                // => manage them manually anyway

                let mut pipeline_layout = vk::PipelineLayout::default();

                for command in batch.commands {

                    match command {
                        Command::BindDescriptorSet { set, block } => {
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
        });
    }

    pub fn draw(&mut self, arg_blocks: &[&ArgumentBlock], vertex_buffers: &[BufferView], index_buffer: Option<BufferView>)
    {
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

    // must either borrow SceneArguments or copy, since we can't create
    let scene_args = ArgumentBlock::new(SceneArguments {
        u_view_matrix: (),
        u_proj_matrix: (),
        u_view_proj_matrix: (),
        u_inverse_proj_matrix: ()
    });

    for batch in material_batches.iter() {
        // argblock borrows image until the draw
        let material_args = ArgumentBlock::new(
            MaterialArguments {
                u_color: (),
                t_color: TextureDescriptor::new(&batch.texture, Sampler::linear())
            });

        for mesh in batch.objects.iter() {

            // Q: is there a memory dependency between the args and the previous draw?
            // -> we don't care, just create a pass on every
            ctx.draw(&[&scene_args, &material_args])
        }
    }
}