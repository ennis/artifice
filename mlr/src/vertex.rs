//! Vertex-related types
use crate::{
    buffer::BufferData,
    vk::{VertexInputAttributeDescription, VertexInputBindingDescription},
};
use graal::vk;
use graal_spirv::typedesc::TypeDesc;

/// Describes the type of indices contained in an index buffer.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum IndexFormat {
    /// 16-bit unsigned integer indices
    U16,
    /// 32-bit unsigned integer indices
    U32,
}

/// Description of a vertex attribute within a vertex buffer layout.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VertexAttribute {
    /// Index of the attribute in the buffer layout.
    index: usize,
    /// Offset of the attribute within a vertex entry.
    offset: usize,
    /// Format of the attribute.
    format: vk::Format,
}

/// Returns the size of bytes of a vertex attribute of the given format.
pub fn vertex_format_byte_size(format: vk::Format) -> usize {
    match format {
        vk::Format::R32G32B32_SFLOAT => 24,
        vk::Format::R32G32B32A32_SFLOAT => 32,
        vk::Format::R32G32_SFLOAT => 16,
        vk::Format::R32_SFLOAT => 8,
        vk::Format::R16G16_UNORM => 4,
        vk::Format::R16G16B16A16_UNORM => 8,
        _ => todo!("unsupported vertex format"),
    }
}

/// Builder for a vertex input layout.
pub struct VertexBufferLayoutBuilder {
    attributes: Vec<VertexAttribute>,
    current_offset: usize,
}

impl VertexBufferLayoutBuilder {
    /// Creates a new builder, starting with no attributes.
    pub fn new() -> VertexBufferLayoutBuilder {
        VertexBufferLayoutBuilder {
            attributes: vec![],
            current_offset: 0,
        }
    }

    /// Pushes a vertex attribute to this buffer layout, and returns the offset and byte size of the
    /// attribute within a vertex element.
    pub fn push_attribute(&mut self, format: vk::Format) -> VertexAttribute {
        let byte_size = vertex_format_byte_size(format);
        let attr = VertexAttribute {
            index: self.attributes.len(),
            offset: byte_size,
            format,
        };
        self.attributes.push(attr);
        // FIXME: alignment
        self.current_offset += byte_size;
        attr
    }

    /// Returns the current stride between two consecutive vertices in the buffer.
    pub fn stride(&self) -> usize {
        self.current_offset
    }

    /// Finishes building the buffer layout and returns it.
    pub fn build(self) -> VertexBufferLayout {
        VertexBufferLayout {
            attributes: self.attributes,
            stride: self.current_offset,
        }
    }
}

/// Layout of a vertex buffer.
pub struct VertexBufferLayout {
    attributes: Vec<VertexAttribute>,
    stride: usize,
}

impl VertexBufferLayout {
    /// Starts building a new vertex buffer layout.
    pub fn build() -> VertexBufferLayoutBuilder {
        VertexBufferLayoutBuilder::new()
    }

    /// Returns the vertex attributes of this layout.
    pub fn attributes(&self) -> &[VertexAttribute] {
        &self.attributes
    }
}
