use crate::{context::Context, TrackingInfo};
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

pub struct ImageAny {
    device: Arc<graal::Device>,
    image: graal::ImageInfo,
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
        ImageAny { device, image }
    }

    pub fn group_id(&self) -> Option<graal::ResourceGroupId> {
        self.device.get_image_state(self.image.id).map(|s| s.group_id)
    }

    pub fn id(&self) -> graal::ImageId {
        self.image.id
    }

    pub(crate) fn resource_id(&self) -> graal::ResourceId {
        self.image.id.resource_id()
    }

    pub fn handle(&self) -> graal::vk::Image {
        self.image.handle
    }
}

impl Drop for ImageAny {
    fn drop(&mut self) {
        self.device.destroy_image(self.image.id)
    }
}
