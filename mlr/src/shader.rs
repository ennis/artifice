//! Shader
use crate::{
    buffer::BufferAny,
    context::{Context, RecordingContext},
    descriptor::DescriptorSetLayoutId,
    image::ImageAny,
    sampler::Sampler,
};
use graal::{
    vk,
    vk::{AccessFlags, ImageLayout, PipelineStageFlags},
    BufferId, ImageId, ResourceGroupId, ResourceId,
};
use graal_spirv::typedesc;
use std::sync::Arc;
use thiserror::Error;

/// Shader arguments (uniforms, textures, etc.).
///
/// Maps to a descriptor set.
/// TODO DOC: descriptor data, layout, update template
pub trait ShaderArguments {
    /// Returns a unique ID for the type of this structure, or None if it's unique.
    fn unique_type_id(&self) -> Option<std::any::TypeId>;

    /// Returns the descriptor set layout for this argument.
    fn get_descriptor_set_layout_bindings(&self) -> &[vk::DescriptorSetLayoutBinding];

    /// Returns the descriptor set update template entries for this argument.
    fn get_descriptor_set_update_template_entries(
        &self,
    ) -> Option<&[vk::DescriptorUpdateTemplateEntry]>;

    /// Updates a descriptor set with the data contained in the arguments.
    unsafe fn update_descriptor_set(
        &mut self,
        ctx: &mut RecordingContext,
        set: vk::DescriptorSet,
        update_template: Option<vk::DescriptorUpdateTemplate>,
    );
}

//Note: `Clone` would require `WithSpan: Clone`.
#[derive(Debug, Error)]
pub enum CreateShaderError {
    #[error(transparent)]
    Vulkan(#[from] vk::Result),
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
                    code_size: spirv.len() * 4,
                    p_code: spirv.as_ptr(),
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
                .as_ref()
                .unwrap()
                .device
                .destroy_shader_module(self.shader_module, None)
        }
    }
}

/// Argument blocks
///
/// Actually they are just descriptor sets.
pub struct ArgumentBlock {
    pub(crate) descriptor_set: vk::DescriptorSet,
}

impl ArgumentBlock {
    /// Creates a new argument block from the specified arguments.
    pub fn new<T: ShaderArguments>(ctx: &mut RecordingContext, mut args: T) -> ArgumentBlock {
        ctx.create_argument_block(args)
    }
}

/*fn test() {
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
        #[argument(sampled_image, binding = 1)]
        t_color: TextureDescriptor<'a>,
    }

    // issue: the draw pass doesn't know the resources used inside
    //

    let scene_args = ctx.create_argument_block(SceneArguments {
        u_view_matrix: (),
        u_proj_matrix: (),
        u_view_proj_matrix: (),
        u_inverse_proj_matrix: (),
    });

    for batch in material_batches.iter() {
        let material_args = ctx.create_argument_block(MaterialArguments {
            u_color: (),
            t_color: TextureDescriptor::new(&batch.texture, Sampler::linear()),
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
*/
