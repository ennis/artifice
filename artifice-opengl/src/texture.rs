use crate::api::gl;
use crate::api::gl::types::*;
use crate::api::Gl;
use crate::gl_format::GlFormatInfoExt;
use artifice_gfxbase::dimensions::Dimensions;
use artifice_gfxbase::format::Format;

//--------------------------------------------------------------------------------------------------
pub struct ExtentsAndType {
    pub target: GLenum,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub array_layers: u32,
}

impl ExtentsAndType {
    pub fn from_dimensions(dim: &Dimensions) -> ExtentsAndType {
        match *dim {
            Dimensions::Dim1d {
                width,
                array_layers,
            } => ExtentsAndType {
                target: gl::TEXTURE_1D,
                width,
                height: 1,
                depth: 1,
                array_layers,
            },
            Dimensions::Dim2d {
                width,
                height,
                array_layers,
            } => ExtentsAndType {
                target: gl::TEXTURE_2D,
                width,
                height,
                depth: 1,
                array_layers,
            },
            Dimensions::Dim3d {
                width,
                height,
                depth,
            } => ExtentsAndType {
                target: gl::TEXTURE_3D,
                width,
                height,
                depth,
                array_layers: 1,
            },
            _ => unimplemented!(),
        }
    }
}

impl_handle_type!(pub struct TextureHandle(GLuint));

impl Drop for TextureHandle {
    fn drop(&mut self) {
        unsafe { self.gl.DeleteTextures(1, &mut self.obj) }
    }
}

/// Wrapper for OpenGL texture objects.
impl TextureHandle {
    pub fn new(
        gl: &Gl,
        format: Format,
        dimensions: &Dimensions,
        mipcount: u32,
        samples: u32,
    ) -> TextureHandle {
        let et = ExtentsAndType::from_dimensions(&dimensions);
        let glfmt = format.gl_format_info();

        if et.array_layers > 1 {
            unimplemented!("array textures")
        }

        let mut obj = 0;
        unsafe {
            gl.CreateTextures(et.target, 1, &mut obj);

            match et.target {
                gl::TEXTURE_1D => {
                    gl.TextureStorage1D(obj, mipcount as i32, glfmt.internal_fmt, et.width as i32);
                }
                gl::TEXTURE_2D => {
                    if samples > 1 {
                        gl.TextureStorage2DMultisample(
                            obj,
                            samples as i32,
                            glfmt.internal_fmt,
                            et.width as i32,
                            et.height as i32,
                            true as u8,
                        );
                    } else {
                        gl.TextureStorage2D(
                            obj,
                            mipcount as i32,
                            glfmt.internal_fmt,
                            et.width as i32,
                            et.height as i32,
                        );
                    }
                }
                gl::TEXTURE_3D => {
                    gl.TextureStorage3D(
                        obj,
                        1,
                        glfmt.internal_fmt,
                        et.width as i32,
                        et.height as i32,
                        et.depth as i32,
                    );
                }
                _ => unimplemented!("texture type"),
            };

            gl.TextureParameteri(obj, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl.TextureParameteri(obj, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl.TextureParameteri(obj, gl::TEXTURE_WRAP_R, gl::CLAMP_TO_EDGE as i32);
            gl.TextureParameteri(obj, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl.TextureParameteri(obj, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

            TextureHandle::from_raw(gl, obj)
        }
    }
}

/// Texture upload
///
/// TODO move in cmd
pub unsafe fn upload_image_region(
    gl: &Gl,
    target: GLenum,
    img: GLuint,
    fmt: Format,
    mip_level: i32,
    offset: (u32, u32, u32),
    size: (u32, u32, u32),
    data: &[u8],
) {
    let fmtinfo = fmt.format_info();
    assert_eq!(
        data.len(),
        (size.0 * size.1 * size.2) as usize * fmtinfo.byte_size(),
        "image data size mismatch"
    );

    // TODO check size of mip level
    let glfmt = fmt.gl_format_info();

    let mut prev_unpack_alignment = 0;
    gl.GetIntegerv(gl::UNPACK_ALIGNMENT, &mut prev_unpack_alignment);
    gl.PixelStorei(gl::UNPACK_ALIGNMENT, 1);

    match target {
        gl::TEXTURE_1D => {
            gl.TextureSubImage1D(
                img,
                mip_level,
                offset.0 as i32,
                size.0 as i32,
                glfmt.upload_components,
                glfmt.upload_ty,
                data.as_ptr() as *const GLvoid,
            );
        }
        gl::TEXTURE_2D => {
            gl.TextureSubImage2D(
                img,
                mip_level,
                offset.0 as i32,
                offset.1 as i32,
                size.0 as i32,
                size.1 as i32,
                glfmt.upload_components,
                glfmt.upload_ty,
                data.as_ptr() as *const GLvoid,
            );
        }
        gl::TEXTURE_3D => {
            gl.TextureSubImage3D(
                img,
                mip_level,
                offset.0 as i32,
                offset.1 as i32,
                offset.2 as i32,
                size.0 as i32,
                size.1 as i32,
                size.2 as i32,
                glfmt.upload_components,
                glfmt.upload_ty,
                data.as_ptr() as *const GLvoid,
            );
        }
        _ => unimplemented!(),
    };

    gl.PixelStorei(gl::UNPACK_ALIGNMENT, prev_unpack_alignment);
}
