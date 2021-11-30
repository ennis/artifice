use crate::{context::Context, TrackingInfo};
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

// buffer on the GPU or CPU OR function result OR static data reference OR local owned data
pub struct BufferAny {
    device: Arc<graal::Device>,
    buffer: graal::BufferInfo,
}

impl BufferAny {
    /// Creates a new, uninitialized resource.
    pub fn new(
        device: &Arc<graal::Device>,
        location: graal::MemoryLocation,
        create_info: graal::BufferResourceCreateInfo,
    ) -> BufferAny {
        let device = device.clone();
        let buffer = device.create_buffer("", location, &create_info);
        BufferAny { device, buffer }
    }

    pub fn group_id(&self) -> Option<graal::ResourceGroupId> {
        self.device
            .get_buffer_state(self.image.id)
            .map(|s| s.group_id)
    }
}

impl Drop for BufferAny {
    fn drop(&mut self) {
        self.device.destroy_buffer(self.buffer.id)
    }
}

pub struct BufferView {}
