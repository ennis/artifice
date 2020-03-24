//! TODO: framebuffer creation
//! impl FramebufferHandle {}
use crate::render::gl::api::gl;
use crate::render::gl::api::Gl;
use crate::render::gl::error::{Error, GlResult};
use crate::render::gl::handle::FramebufferHandle;
use crate::render::gl::handle::RenderbufferHandle;
use crate::render::gl::handle::TextureHandle;

#[derive(Copy, Clone)]
pub enum FramebufferAttachment<'a> {
    Texture(&'a TextureHandle),
    Renderbuffer(&'a RenderbufferHandle),
}

impl FramebufferHandle {
    pub unsafe fn new(
        gl: &Gl,
        color_attachments: &[FramebufferAttachment],
        depth_attachment: Option<FramebufferAttachment>,
    ) -> GlResult<FramebufferHandle> {
        assert!(color_attachments.len() < 8);

        let mut obj = 0;
        gl.CreateFramebuffers(1, &mut obj);

        // color attachments
        for (index, a) in color_attachments.iter().enumerate() {
            let index = index as u32;
            match a {
                FramebufferAttachment::Renderbuffer(r) => {
                    gl.NamedFramebufferRenderbuffer(
                        obj,
                        gl::COLOR_ATTACHMENT0 + index,
                        gl::RENDERBUFFER,
                        r.obj,
                    );
                }
                FramebufferAttachment::Texture(tex) => {
                    gl.NamedFramebufferTexture(
                        obj,
                        gl::COLOR_ATTACHMENT0 + index,
                        tex.obj,
                        0, // TODO
                    );
                }
            }
        }

        // depth-stencil attachment
        if let Some(a) = depth_attachment {
            match a {
                FramebufferAttachment::Renderbuffer(r) => {
                    gl.NamedFramebufferRenderbuffer(
                        obj,
                        gl::DEPTH_ATTACHMENT,
                        gl::RENDERBUFFER,
                        r.obj,
                    );
                }
                FramebufferAttachment::Texture(tex) => {
                    gl.NamedFramebufferTexture(
                        obj,
                        gl::DEPTH_ATTACHMENT,
                        tex.obj,
                        0, // TODO
                    );
                }
            }
        }

        // enable draw buffers
        gl.NamedFramebufferDrawBuffers(
            obj,
            color_attachments.len() as i32,
            [
                gl::COLOR_ATTACHMENT0,
                gl::COLOR_ATTACHMENT0 + 1,
                gl::COLOR_ATTACHMENT0 + 2,
                gl::COLOR_ATTACHMENT0 + 3,
                gl::COLOR_ATTACHMENT0 + 4,
                gl::COLOR_ATTACHMENT0 + 5,
                gl::COLOR_ATTACHMENT0 + 6,
                gl::COLOR_ATTACHMENT0 + 7,
            ]
            .as_ptr(),
        );

        // check framebuffer completeness
        let status = gl.CheckNamedFramebufferStatus(obj, gl::DRAW_FRAMEBUFFER);

        if status == gl::FRAMEBUFFER_COMPLETE {
            Ok(FramebufferHandle::from_raw(gl, obj))
        } else {
            Err(Error::FramebufferIncomplete(status))
        }
    }
}
