use crate::context::Context;
use std::sync::Arc;

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
}

impl Drop for ImageAny {
    fn drop(&mut self) {
        self.device.destroy_image(self.image.id)
    }
}
