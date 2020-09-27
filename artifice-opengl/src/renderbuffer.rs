use crate::api::Gl;
use crate::api::gl::types::*;
use crate::gl_format::GlFormatInfoExt;
use crate::framebuffer::FramebufferAttachment::Renderbuffer;
use artifice_gfxbase::format::Format;
use artifice_gfxbase::dimensions::Dimensions;
use crate::texture::ExtentsAndType;

impl_handle_type!(pub struct RenderbufferHandle(GLuint));

impl Drop for RenderbufferHandle {
    fn drop(&mut self) {
        unsafe { self.gl.DeleteRenderbuffers(1, &mut self.obj) }
    }
}

impl RenderbufferHandle
{
    pub fn new(
        gl: &Gl,
        format: Format,
        dimensions: &Dimensions,
        samples: u32,
    ) -> RenderbufferHandle {
        let et = ExtentsAndType::from_dimensions(&dimensions);
        let glfmt = format.gl_format_info();

        let mut obj = 0;

        unsafe {
            gl.CreateRenderbuffers(1, &mut obj);

            if samples > 1 {
                gl.NamedRenderbufferStorageMultisample(
                    obj,
                    samples as i32,
                    glfmt.internal_fmt,
                    et.width as i32,
                    et.height as i32,
                );
            } else {
                gl.NamedRenderbufferStorage(
                    obj,
                    glfmt.internal_fmt,
                    et.width as i32,
                    et.height as i32,
                );
            }

            RenderbufferHandle::from_raw(gl, obj)
        }
    }
}