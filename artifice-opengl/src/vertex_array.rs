use crate::gl_format::{GlFormatInfo, GlFormatInfoExt};
use crate::api::Gl;
use crate::api::gl::types::*;
use artifice_gfxbase::vertex::{VertexData, VertexLayout};

// vertex array: describes the layout of inputs to the vertex processing stage

impl_handle_type!(pub struct VertexArrayHandle(GLuint));

impl Drop for VertexArrayHandle {
    fn drop(&mut self) {
        unsafe { self.gl.DeleteVertexArrays(1, &mut self.obj) }
    }
}

impl VertexArrayHandle {
    // don't know yet how and when we will create vertex arrays, so don't

    pub fn from_vertex_data<T: VertexData>(gl: &Gl) -> VertexArrayHandle {
        VertexArrayHandle::new(gl, &[&T::LAYOUT])
    }

    pub fn new(gl: &Gl, buffer_layouts: &[&VertexLayout]) -> VertexArrayHandle {
        let mut vao = 0;
        let mut loc_ctr = 0;
        unsafe {
            gl.CreateVertexArrays(1, &mut vao);

            for (binding,buffer_layout) in buffer_layouts.iter().enumerate()
            {
                for element in buffer_layout.elements {
                    // determine the location of the attrib
                    let index = if let Some(semantic) = element.semantic {
                        // use the provided location
                        // update the counter
                        loc_ctr = semantic.index;
                        semantic.index
                    } else {
                        // use the automatic location counter
                        let l = loc_ctr;
                        loc_ctr += 1;
                        l
                    };

                    gl.EnableVertexArrayAttrib(vao, index);
                    let fmtinfo = element.format.format_info();
                    let normalized = fmtinfo.is_normalized() as u8;
                    let size = fmtinfo.num_components() as i32;
                    let glfmt = element.format.gl_format_info();
                    let ty = glfmt.upload_ty;

                    gl.VertexArrayAttribFormat(vao, index, size, ty, normalized, element.offset);
                    gl.VertexArrayAttribBinding(vao, index, binding as u32);
                }
            }

            VertexArrayHandle::from_raw(gl, vao)
        }
    }

    /* pub unsafe fn new(gl: &Gl, attribs: &[VertexInputAttributeDescription]) -> VertexArrayHandle {

        let mut vao = 0;
        gl.CreateVertexArrays(1, &mut vao);

        for a in attribs.iter() {
            gl.EnableVertexArrayAttrib(vao, a.location);
            let fmtinfo = a.format.get_format_info();
            let normalized = fmtinfo.is_normalized() as u8;
            let size = fmtinfo.num_components() as i32;
            let glfmt = GlFormatInfo::from_format(a.format);
            let ty = glfmt.upload_ty;

            gl.VertexArrayAttribFormat(vao, a.location, size, ty, normalized, a.offset);
            gl.VertexArrayAttribBinding(vao, a.location, a.binding);
        }

        vao
    }*/
}
