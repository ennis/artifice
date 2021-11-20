use crate::context::Context;
use std::sync::Arc;

// issue: usage flags must be set beforehand?

pub struct ImageAny {
    device: Arc<graal::Device>,
    image: graal::ImageInfo,
    group: Option<graal::ResourceGroupId>,
}

impl ImageAny {
    /// Creates a new, uninitialized resource.
    pub fn new(
        context: &Context,
        location: graal::MemoryLocation,
        create_info: graal::ImageResourceCreateInfo,
    ) -> ImageAny {
        let device = context.device().clone();
        let image = device.create_image("", location, &create_info);
        ImageAny {
            device,
            image,
            group: None,
        }
    }

    /// Returns the group that this resource belongs to, or None if it doesn't belong to any resource
    /// group.
    pub fn resource_group(&self) -> Option<graal::ResourceGroupId> {
        self.group
    }
}

impl Drop for ImageAny {
    fn drop(&mut self) {
        self.device.destroy_image(self.id)
    }
}
