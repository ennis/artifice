//! Shader
use graal::{
    vk,
    vk::{AccessFlags, ImageLayout, PipelineStageFlags},
    BufferId, ImageId, ResourceGroupId, ResourceId,
};
use graal_spirv::typedesc;
use std::sync::Arc;
use thiserror::Error;

/// Error during shader module creation.
#[derive(Debug, Error)]
pub enum CreateShaderError {
    #[error(transparent)]
    Vulkan(#[from] vk::Result),
}

/// Wrapper over a vulkan ShaderModule.
pub struct ShaderModule {
    device: Arc<graal::Device>,
    pub(crate) shader_module: vk::ShaderModule,
}

impl ShaderModule {
    /// Creates a new shader from SPIR-V bytecode.
    pub fn from_spirv(device: &Arc<graal::Device>, spirv: &[u32]) -> Result<ShaderModule, CreateShaderError> {
        let device = device.clone();
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

        Ok(ShaderModule { device, shader_module })
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            // Safety: we control the shader module at all times.
            self.device.device.destroy_shader_module(self.shader_module, None)
        }
    }
}
