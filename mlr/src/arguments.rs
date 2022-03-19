use crate::{buffer::BufferAny, image::ImageAny, sampler::SamplerType, vk};
use graal::{Device, PassBuilder};
use graal_spirv::typedesc::TypeDesc;
use kyute_common::Atom;
use std::{ptr, sync::Arc};

/// Description of a variable within a uniform buffer.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct UniformDesc {
    binding: usize,
    offset: usize,
    size: usize,
}

/// Builder of ArgumentBlockLayouts.
pub struct ArgumentBlockLayoutBuilder {
    uniform_buffer_layout: Vec<UniformDesc>,
    //textures:
}

impl ArgumentBlockLayoutBuilder {
    pub fn new() -> ArgumentBlockLayoutBuilder {
        ArgumentBlockLayoutBuilder {
            uniform_buffer_layout: vec![],
        }
    }

    /// Adds a uniform variable to the argument block with the specified type.
    ///
    /// Internally, it adds an entry to the default uniform buffer at binding #0.
    pub fn add_uniform(&mut self, name: Atom, ty: TypeDesc) {
        todo!()
    }

    /// Adds a uniform buffer input.
    pub fn add_uniform_buffer(&mut self, ty: TypeDesc) -> UniformDesc {
        todo!()
    }

    pub fn build(self) -> ArgumentBlockLayout {
        // binding #0 is uniform buffer
        let ubo = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: Default::default(),
            descriptor_count: 0,
            stage_flags: vk::ShaderStageFlags::ALL, // TODO
            p_immutable_samplers: ptr::null(),
        };

        // binding #1-#n are textures
        //let mut bindings =

        let create_info = vk::DescriptorSetLayoutCreateInfo {
            flags: Default::default(),
            binding_count: 0,
            p_bindings: (),
            ..Default::default()
        };
    }
}

/// Describes the layout of an arguments block (equivalent to a descriptor set).
pub struct ArgumentBlockLayout {
    device: Arc<graal::Device>,
    descriptor_set_layout: vk::DescriptorSetLayout,
}

impl ArgumentBlockLayout {
    pub fn builder() -> ArgumentBlockLayoutBuilder {
        ArgumentBlockLayoutBuilder::new()
    }
}

/// Builder for an argument block.
pub struct ArgumentBlockBuilder {
    layout: Arc<ArgumentBlockLayout>,
}

impl ArgumentBlockBuilder {
    pub fn new(layout: Arc<ArgumentBlockLayout>) -> ArgumentBlockBuilder {
        ArgumentBlockBuilder { layout }
    }

    // how to build an argument block?
    // -> progressively specify all uniform variables, textures, and everything
    pub unsafe fn set_uniform<T: Copy>(&mut self, index: usize, val: T) {
        // check that size matches, at least, then copy into buffer
    }
}

pub struct ArgumentBlock {}

impl ArgumentBlock {
    pub fn builder(layout: Arc<ArgumentBlockLayout>) -> ArgumentBlock {
        ArgumentBlockBuilder::new(layout)
    }
}

/// Trait implemented by types that hold references to resources.
pub trait ResourceAccess {
    /// Visits all resources referenced by this object.
    fn register(&self, pass: &mut graal::PassBuilder<()>);
}

/// Arguments with statically known descriptor set layout.
pub trait StaticArguments {
    const TYPE_ID: std::any::TypeId;
    const LAYOUT: &'static [vk::DescriptorSetLayoutBinding];
}

/// Shader arguments (uniforms, textures, etc.).
pub trait Arguments: ResourceAccess {
    /// Returns a unique ID for the type of this structure, or None if it's unique.
    fn unique_type_id(&self) -> Option<std::any::TypeId>;

    /// Returns the descriptor set layout for this argument.
    fn get_descriptor_set_layout_bindings(&self) -> &[vk::DescriptorSetLayoutBinding];

    /// Updates a descriptor set with the data contained in the arguments.
    unsafe fn update_descriptor_set(
        &mut self,
        device: &graal::Device,
        descriptor_set_builder: &mut DescriptorSetBuilder,
        update_template: Option<vk::DescriptorUpdateTemplate>,
    );
}

pub struct DescriptorSetBuilder {
    set: vk::DescriptorSet,
    writes: Vec<vk::WriteDescriptorSet>,
    image_infos: Vec<vk::DescriptorImageInfo>,
    buffer_infos: Vec<vk::DescriptorBufferInfo>,
    texel_buffer_views: Vec<vk::BufferView>,
}

impl DescriptorSetBuilder {
    fn new(set: vk::DescriptorSet) -> DescriptorSetBuilder {
        DescriptorSetBuilder {
            set,
            writes: vec![],
            image_infos: vec![],
            buffer_infos: vec![],
            texel_buffer_views: vec![],
        }
    }

    pub fn write_image_descriptor(
        &mut self,
        binding: u32,
        array_element: u32,
        count: u32,
        descriptor_type: vk::DescriptorType,
        image_info: vk::DescriptorImageInfo,
    ) {
        self.image_infos.push(image_info);
        let offset = self.image_infos.len() - 1;
        self.writes.push(vk::WriteDescriptorSet {
            dst_set: self.set,
            dst_binding: binding,
            dst_array_element: array_element,
            descriptor_count: count,
            descriptor_type,
            p_image_info: offset as *const _,
            p_buffer_info: ptr::null(),
            p_texel_buffer_view: ptr::null(),
            ..Default::default()
        })
    }

    pub fn write_buffer_descriptor(
        &mut self,
        binding: u32,
        array_element: u32,
        count: u32,
        descriptor_type: vk::DescriptorType,
        buffer_info: vk::DescriptorBufferInfo,
    ) {
        self.buffer_infos.push(buffer_info);
        let offset = self.buffer_infos.len() - 1;
        self.writes.push(vk::WriteDescriptorSet {
            dst_set: self.set,
            dst_binding: binding,
            dst_array_element: array_element,
            descriptor_count: count,
            descriptor_type,
            p_image_info: ptr::null(),
            p_buffer_info: offset as *const _,
            p_texel_buffer_view: ptr::null(),
            ..Default::default()
        })
    }

    fn finish(mut self, device: &graal::Device) {
        unsafe {
            for w in self.writes.iter_mut() {
                if !w.p_image_info.is_null() {
                    w.p_image_info = self.image_infos.as_ptr().add(w.p_image_info as usize);
                } else if !w.p_buffer_info.is_null() {
                    w.p_buffer_info = self.buffer_infos.as_ptr().add(w.p_buffer_info as usize);
                } else if !w.p_texel_buffer_view.is_null() {
                    w.p_texel_buffer_view = self.texel_buffer_views.as_ptr().add(w.p_texel_buffer_view as usize);
                }
            }
            device.device.update_descriptor_sets(&self.writes, &[]);
        }
    }
}

///
pub unsafe trait DescriptorBinding: ResourceAccess {
    /// Descriptor type.
    const DESCRIPTOR_TYPE: vk::DescriptorType;
    /// Which shader stages can access a resource for this binding.
    const SHADER_STAGES: vk::ShaderStageFlags;
    /// Number of descriptors represented in this object.
    const DESCRIPTOR_COUNT: u32;

    /// Prepares the descriptor update data during pass evaluation.
    ///
    /// # Arguments
    ///
    ///
    /// # Note
    /// Implementations can access the submission context to upload uniform data, create image views,
    /// create samplers, etc.
    /// This cannot be done before evaluation since resources may not have memory bound to them at that point.
    fn write_descriptors(
        &self,
        device: &graal::Device,
        binding: u32,
        descriptor_set_builder: &mut DescriptorSetBuilder,
    );
}

//--------------------------------------------------------------------------------------------------

/// Sampled image descriptor.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct SampledImage2D<'a> {
    pub(crate) image: &'a ImageAny,
}

unsafe impl<'a> DescriptorBinding for SampledImage2D<'a> {
    const DESCRIPTOR_TYPE: vk::DescriptorType = vk::DescriptorType::SAMPLED_IMAGE;
    const SHADER_STAGES: vk::ShaderStageFlags = vk::ShaderStageFlags::ALL;
    const DESCRIPTOR_COUNT: u32 = 1;

    fn write_descriptors(
        &self,
        device: &graal::Device,
        binding: u32,
        descriptor_set_builder: &mut DescriptorSetBuilder,
    ) {
        // SAFETY: TODO
        let image_view = unsafe {
            let create_info = vk::ImageViewCreateInfo {
                flags: vk::ImageViewCreateFlags::empty(),
                image: self.image.handle(),
                view_type: vk::ImageViewType::TYPE_2D,
                format: self.image.format(),
                components: vk::ComponentMapping::default(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: vk::REMAINING_MIP_LEVELS,
                    base_array_layer: 0,
                    layer_count: vk::REMAINING_ARRAY_LAYERS,
                },
                ..Default::default()
            };
            /// FIXME: this should probably be cached into the image
            let image_view = device
                .device
                .create_image_view(&create_info, None)
                .expect("could not create image view");
            // immediately schedule deletion since it will be used only in this frame
            device.destroy_image_view(image_view);
            image_view
        };

        descriptor_set_builder.write_image_descriptor(
            binding,
            0,
            1,
            vk::DescriptorType::SAMPLED_IMAGE,
            vk::DescriptorImageInfo {
                sampler: Default::default(),
                image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
        );
    }
}

impl<'a> ResourceAccess for SampledImage2D<'a> {
    fn register(&self, pass: &mut PassBuilder<()>) {
        pass.add_image_dependency(
            self.image.image.id,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )
    }
}

impl<'a> From<&'a ImageAny> for SampledImage2D<'a> {
    fn from(img: &'a ImageAny) -> Self {
        img.to_sampled_image_2d()
    }
}

//--------------------------------------------------------------------------------------------------

/// Combined image/sampler descriptor.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct CombinedImageSampler2D<'a, S: SamplerType> {
    pub(crate) image: &'a ImageAny,
    pub(crate) sampler: S,
}

unsafe impl<'a, S: SamplerType> DescriptorBinding for CombinedImageSampler2D<'a, S> {
    const DESCRIPTOR_TYPE: vk::DescriptorType = vk::DescriptorType::COMBINED_IMAGE_SAMPLER;
    const SHADER_STAGES: vk::ShaderStageFlags = vk::ShaderStageFlags::ALL;
    const DESCRIPTOR_COUNT: u32 = 1;

    fn write_descriptors(
        &self,
        device: &graal::Device,
        binding: u32,
        descriptor_set_builder: &mut DescriptorSetBuilder,
    ) {
        // SAFETY: TODO
        let image_view = unsafe {
            let create_info = vk::ImageViewCreateInfo {
                flags: vk::ImageViewCreateFlags::empty(),
                image: self.image.handle(),
                view_type: vk::ImageViewType::TYPE_2D,
                format: self.image.format(),
                components: vk::ComponentMapping::default(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: vk::REMAINING_MIP_LEVELS,
                    base_array_layer: 0,
                    layer_count: vk::REMAINING_ARRAY_LAYERS,
                },
                ..Default::default()
            };
            /// FIXME: this should probably be cached into the image
            let image_view = device
                .device
                .create_image_view(&create_info, None)
                .expect("could not create image view");
            // immediately schedule deletion since it will be used only in this frame
            device.destroy_image_view(image_view);
            image_view
        };

        let sampler = self.sampler.to_sampler(&device.device);

        descriptor_set_builder.write_image_descriptor(
            binding,
            0,
            1,
            vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            vk::DescriptorImageInfo {
                sampler,
                image_view,
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            },
        );
    }
}

impl<'a, S: SamplerType> ResourceAccess for CombinedImageSampler2D<'a, S> {
    fn register(&self, pass: &mut PassBuilder<()>) {
        pass.add_image_dependency(
            self.image.image.id,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::ALL_COMMANDS,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )
    }
}

//--------------------------------------------------------------------------------------------------

/// Uniform buffer slice.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct UniformBuffer<'a> {
    pub(crate) buffer: &'a BufferAny,
    pub(crate) offset: vk::DeviceSize,
    pub(crate) range: vk::DeviceSize,
}

unsafe impl<'a> DescriptorBinding for UniformBuffer<'a> {
    const DESCRIPTOR_TYPE: vk::DescriptorType = vk::DescriptorType::UNIFORM_BUFFER;
    const SHADER_STAGES: vk::ShaderStageFlags = vk::ShaderStageFlags::ALL;
    const DESCRIPTOR_COUNT: u32 = 1;

    fn write_descriptors(&self, device: &Device, binding: u32, descriptor_set_builder: &mut DescriptorSetBuilder) {
        descriptor_set_builder.write_buffer_descriptor(
            binding,
            0,
            1,
            vk::DescriptorType::UNIFORM_BUFFER,
            vk::DescriptorBufferInfo {
                buffer: self.buffer.handle(),
                offset: self.offset,
                range: self.range,
            },
        );
    }
}

impl<'a> ResourceAccess for UniformBuffer<'a> {
    fn register(&self, pass: &mut PassBuilder<()>) {
        pass.add_buffer_dependency(
            self.buffer.id(),
            vk::AccessFlags::UNIFORM_READ,
            vk::PipelineStageFlags::ALL_COMMANDS,
        )
    }
}

//--------------------------------------------------------------------------------------------------
