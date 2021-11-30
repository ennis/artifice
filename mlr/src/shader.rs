//! Shader
use std::any::TypeId;
use std::cell::Cell;
use std::collections::HashSet;
use std::num::NonZeroU32;
use crate::context::Context;
use graal::{BufferId, ImageId, ResourceGroupId, ResourceId, vk};
use graal_spirv::typedesc;
use std::sync::Arc;
use thiserror::Error;
use graal::vk::{AccessFlags, ImageLayout, PipelineStageFlags};
use crate::{Buffer, BufferView};
use crate::buffer::BufferAny;
use crate::descriptor::{DescriptorSetLayoutCache, DescriptorSetLayoutId};
use crate::frame::{Arena, PassSubmitCtx};
use crate::image::ImageAny;
use crate::sampler::Sampler;



pub trait ResourceVisitor {
    // TODO only one function instead?
    fn visit_image(&mut self, image: &ImageAny,
                   access_mask: graal::vk::AccessFlags,
                   stage_mask: graal::vk::PipelineStageFlags,
                   layout: graal::vk::ImageLayout) -> bool;
    fn visit_buffer(&mut self, buffer: &BufferAny,
                    access_mask: graal::vk::AccessFlags,
                    stage_mask: graal::vk::PipelineStageFlags) -> bool;
}

/// Trait implemented by types that hold references to resources.
pub trait ResourceHolder {
    /// Visits all resources referenced by this object.
    fn walk_resources(&self, visitor: &mut dyn ResourceVisitor);

    /// Returns whether this object only holds read-only references to resources.
    fn is_read_only(&self) -> bool;
}

/// Shader arguments (uniforms, textures, etc.).
///
/// Maps to a descriptor set.
/// TODO DOC: descriptor data, layout, update template
pub trait ShaderArguments: ResourceHolder {
    /// Returns the descriptor set layout for this argument.
    fn get_or_create_descriptor_set_layout(&self, cache: &mut DescriptorSetLayoutCache) -> DescriptorSetLayoutId;

    /// Updates a descriptor set with the data contained in the arguments.
    unsafe fn update_descriptor_set(&self,
                             ctx: &mut PassSubmitCtx,
                             device: &graal::ash::Device,
                             set: vk::DescriptorSet,
                             update_template: Option<vk::DescriptorUpdateTemplate>);
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


#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct ArgumentBlockId(pub(crate) usize);

/// Argument blocks
pub struct ArgumentBlock<'a, T: ?Sized + ShaderArguments + 'a> {
    /// Index uniquely identifying this argument block in the current frame.
    pub(crate) id: Cell<Option<NonZeroU32>>,
    pub(crate) last_pass_id: Cell<Option<NonZeroU32>>,
    /// Arguments
    pub(crate) args: T,
}

impl<'a, T: ShaderArguments + 'a> ArgumentBlock<'a, T>
{
    /// Creates a new argument block from the specified arguments.
    pub fn new(args: T) -> ArgumentBlock<'a, T> {
        ArgumentBlock {
            id: Cell::new(None),
            args,
        }
    }
}

impl<'a, T: ?Sized + ShaderArguments + 'a> ArgumentBlock<'a, T> {
    pub(crate) fn walk_resources(&self, visitor: &mut dyn ResourceVisitor) {
        self.args.walk_resources(visitor)
    }

    /// Writes the descriptor data into the provided arena.
    pub(crate) fn write_descriptor_data<'a>(&self, arena: &'a Arena) -> &'a [u8] {
        self.args.write_descriptor_update_data(arena)
    }

    /// Returns the descriptor set layout associated to this argument block.
    pub(crate) fn get_or_create_descriptor_set_layout(&self, cache: &mut DescriptorSetLayoutCache) -> DescriptorSetLayoutId {
        self.args.get_or_create_descriptor_set_layout(cache)
    }

    /*pub(crate) fn set_frame_id(&self, id: ArgumentBlockId) {
        self.frame_id.set(Some(id))
    }*/
}


impl Context {

    fn flush_pending_draw_calls(&mut self) {
        /*let descriptor_cache = self.descriptor_cache.clone();
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

        self.context.end_pass();*/
    }

    // dyn ArgumentBlock
    //      args.visit(dyn ArgsVisitor)

    pub fn draw(&mut self, arg_blocks: &[&dyn ArgumentBlockBase], vertex_buffers: &[BufferView], index_buffer: Option<BufferView>)
    {


        // issue: given argument blocks, vertex buffers, and other resources,
        // determine if we should stay on this pass or flush it and open a new one
        //
        // We must end the current pass if:
        // - we introduce a write dependency that wasn't there before
        // - and that's it? we can introduce additional read deps without the need to start another pass
        //      - syncing too eagerly on reads may become a problem
        //      -> in this case, just use a render graph

        // problem: we now have two ways of submitting draw commands:
        // - "immediate" mode, in which the passes are inferred on-the-fly
        //      - needs to track used resources: Vec<ResourceDependency> => one allocation per argblocks
        // - "render graph" mode, in which the draw commands are submitted directly to command buffers
        //      - doesn't need to track used resources
        // argblocks: for immediate mode, they need to also store the used resources; for rendergraph mode, there's no need,
        // since sync is handled by the rendergraph system.
        // => argblocks have overhead in "rendergraph" mode
        //
        //
        // Argblocks are supposed to be cheap; ideally, no dynamic allocation.
        // Can argblocks survive across frames?
        // => typed argblocks
        // => they hold a copy of the ShaderArguments
        // => implements argblock interface or whatever




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


    // issue: the draw pass doesn't know the resources used inside
    //

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

    // other solution: create argument blocks during command buffer generation

    // problem: draw pass must specify dependencies
    // problem: command generation callback must borrow stuff (scene?)


}