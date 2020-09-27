use crate::api::gl::types::GLenum;

#[derive(Clone, Debug)]
pub enum Error {
    FramebufferIncomplete(GLenum),
    ShaderCompilationError(String),
    ProgramLinkError(String),
}

pub type GlResult<T> = Result<T, Error>;
