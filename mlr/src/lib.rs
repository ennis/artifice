pub mod buffer;
pub mod context;
pub mod descriptor;
pub mod frame;
pub mod image;
//pub mod pipeline;
pub mod sampler;
pub mod shader;
pub mod utils;
pub mod vertex;

// macro support
extern crate self as mlr;

pub use graal::vk;
pub use mlr_macros::{ShaderArguments, StructLayout, VertexData};
pub use shader::ShaderArguments;
pub use descriptor::DescriptorBinding;
pub use vertex::{VertexAttribute, VertexData};

/*#[derive(Copy, Clone, Debug)]
pub(crate) struct TrackingInfo {
    pub(crate) read: u64,
    pub(crate) write: u64,
    pub(crate) access_mask: graal::vk::AccessFlags,
    pub(crate) layout: graal::vk::ImageLayout,
}

impl TrackingInfo {
    pub fn initial() -> TrackingInfo {
        TrackingInfo {
            read: 0,
            write: 0,
            access_mask: graal::vk::AccessFlags::empty(),
            layout: graal::vk::ImageLayout::UNDEFINED,
        }
    }

    /// Returns whether a WAW/RAW/WAR hazard is possible.
    pub(crate) fn update_access(&mut self, sn: u64, access_mask: vk::AccessFlags, layout: vk::ImageLayout) -> bool {
        let mut hazard = false;
        if graal::is_write_access(access_mask) || layout != self.layout {
            if (self.read == sn)
                || (self.write == sn && (access_mask != self.access_mask || access_mask != vk::AccessFlags::COLOR_ATTACHMENT_WRITE))
                || (self.write == sn && layout != self.layout) {
                hazard = true;
            }
            self.write = sn;
            self.read = sn;
            self.layout = layout;
            self.access_mask = access_mask;
        } else {
            // read access
            if self.write == sn {
                // last write was in the current pass: RAW hazard
                hazard = true;
            }
            self.read = sn;
        }
        hazard
    }
}*/

/*struct ContextCache<T> {
    /// uniquely identifies the context that put the value there
    context_id: u64,
    value: RefCell<Option<T>>,
}

struct ShaderFunction {
    // GLSL source
// unique ID/hash
// uniforms
}

struct ColorTargets {
    // target images
}
*/
