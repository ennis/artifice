use std::sync::Arc;

/// Marker trait for data that can be uploaded to a GPU buffer
pub trait BufferData: 'static {
    type Element;
    //fn len(&self) -> usize;
}

impl<T: Copy + 'static> BufferData for T {
    type Element = T;
    //fn len(&self) -> usize {
    //    1
    //}
}

impl<U: BufferData> BufferData for [U] {
    type Element = U;
    //fn len(&self) -> usize {
    //    (&self as &[U]).len()
    //}
}

// buffer on the GPU or CPU OR function result OR static data reference OR local owned data
#[derive(Debug)]
pub struct BufferAny {
    device: Arc<graal::Device>,
    buffer: graal::BufferInfo,
}

impl BufferAny {
    /// Creates a new, uninitialized buffer resource.
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
        self.device.get_buffer_state(self.buffer.id).map(|s| s.group_id)
    }

    /// Returns the vulkan handle (`VkBuffer`) of this buffer.
    pub fn handle(&self) -> graal::vk::Buffer {
        self.buffer.handle
    }

    /// Returns the vulkan handle (`VkBuffer`) of this buffer.
    pub fn id(&self) -> graal::BufferId {
        self.buffer.id
    }
}

impl Drop for BufferAny {
    fn drop(&mut self) {
        self.device.destroy_buffer(self.buffer.id)
    }
}
