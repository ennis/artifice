//! TODO: framebuffer creation
//! impl FramebufferHandle {}
use crate::api::Gl;
use crate::api::gl;
use crate::api::gl::types::*;
use crate::error::{GlResult, Error};
use crate::texture::TextureHandle;
use crate::renderbuffer::RenderbufferHandle;

impl_handle_type!(pub struct FramebufferHandle(GLuint));

impl Drop for FramebufferHandle {
    fn drop(&mut self) {
        unsafe { self.gl.DeleteFramebuffers(1, &mut self.obj) }
    }
}

#[derive(Copy, Clone)]
pub enum FramebufferAttachment<'a> {
    Texture(&'a TextureHandle),
    Renderbuffer(&'a RenderbufferHandle),
}

#[derive(Default)]
pub struct FramebufferBuilder<'a> {
    color_attachments: Vec<FramebufferAttachment<'a>>,
    depth_attachment: Option<FramebufferAttachment<'a>>
}

impl<'a> FramebufferBuilder<'a> {
    pub fn color_texture(&mut self, texture: &'a TextureHandle) -> &mut Self {
        self.color_attachments.push(FramebufferAttachment::Texture(texture));
        self
    }

    pub fn color_renderbuffer(&mut self, renderbuffer: &'a RenderbufferHandle) -> &mut Self {
        self.color_attachments.push(FramebufferAttachment::Renderbuffer(renderbuffer));
        self
    }

    pub fn depth_texture(&mut self, texture: &'a TextureHandle) -> &mut Self {
        self.depth_attachment = Some(FramebufferAttachment::Texture(texture));
        self
    }

    pub fn depth_renderbuffer(&mut self, renderbuffer: &'a RenderbufferHandle) -> &mut Self {
        self.depth_attachment = Some(FramebufferAttachment::Renderbuffer(renderbuffer));
        self
    }

    pub unsafe fn build(&mut self, gl: &Gl) -> GlResult<FramebufferHandle> {
        FramebufferHandle::new(gl, &self.color_attachments, self.depth_attachment)
    }
}

impl FramebufferHandle {
    pub fn builder<'a>() -> FramebufferBuilder<'a> {
        FramebufferBuilder {
            color_attachments: Vec::new(),
            depth_attachment: None
        }
    }

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
