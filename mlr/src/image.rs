use crate::{arguments::SampledImage2D, context::Context, sampler::SamplerType, vk};
use mlr::arguments::CombinedImageSampler2D;
use std::sync::Arc;

#[derive(Debug)]
pub struct ImageAny {
    device: Arc<graal::Device>,
    image: graal::ImageInfo,
    format: graal::vk::Format,
}

impl ImageAny {
    /// Creates a new, uninitialized resource.
    pub fn new(
        device: &Arc<graal::Device>,
        location: graal::MemoryLocation,
        create_info: graal::ImageResourceCreateInfo,
    ) -> ImageAny {
        let device = device.clone();
        let image = device.create_image("", location, &create_info);
        ImageAny {
            device,
            image,
            format: create_info.format,
        }
    }

    /// Returns the ID of the resource group that this image belongs to.
    pub fn group_id(&self) -> Option<graal::ResourceGroupId> {
        self.device
            .get_image_state(self.image.id)
            .map(|s| s.group_id)
    }

    /// Returns the backend ID of this image.
    pub fn id(&self) -> graal::ImageId {
        self.image.id
    }

    /// Returns the backend resource ID of this image.
    pub(crate) fn resource_id(&self) -> graal::ResourceId {
        self.image.id.resource_id()
    }

    /// Returns the vulkan handle (`VkImage`) of this image.
    pub fn handle(&self) -> graal::vk::Image {
        self.image.handle
    }

    /// Returns the format of this image.
    pub fn format(&self) -> graal::vk::Format {
        self.format
    }

    ///
    pub fn to_sampled_image_2d(&self) -> SampledImage2D {
        SampledImage2D {
            image: self,
            // leave uninitialized
            descriptor: vk::DescriptorImageInfo {
                sampler: Default::default(),
                image_view: Default::default(),
                image_layout: Default::default(),
            },
        }
    }

    pub fn to_combined_image_sampler_2d<S: SamplerType>(
        &self,
        sampler: S,
    ) -> CombinedImageSampler2D<S> {
        CombinedImageSampler2D {
            image: self,
            sampler,
            // leave uninitialized
            descriptor: vk::DescriptorImageInfo {
                sampler: Default::default(),
                image_view: Default::default(),
                image_layout: Default::default(),
            },
        }
    }
}

impl Drop for ImageAny {
    fn drop(&mut self) {
        self.device.destroy_image(self.image.id)
    }
}
