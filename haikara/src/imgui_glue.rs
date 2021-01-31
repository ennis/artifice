//! Renderer for dear imgui (https://github.com/ocornut/imgui).
use artifice_gfxbase::dimensions::Dimensions;
use artifice_gfxbase::format::Format;
use artifice_gfxbase::format::Format::R8G8B8A8_SNORM;
use artifice_gfxbase::vertex::{Semantic, VertexData, VertexLayout, VertexLayoutElement};
use artifice_opengl::api::gl;
use artifice_opengl::api::gl::types::*;
use artifice_opengl::api::Gl;
use artifice_opengl::buffer::BufferHandle;
use artifice_opengl::shader::{FragmentShaderHandle, ProgramHandle, VertexShaderHandle};
use artifice_opengl::texture::TextureHandle;
use artifice_opengl::vertex_array::VertexArrayHandle;
use imgui::internal::RawWrapper;
use imgui::DrawCmd;
use std::ffi::c_void;
use std::{mem, slice};

static IMGUI_VERTEX_SHADER_SOURCE: &str = include_str!("imgui.vert");
static IMGUI_FRAGMENT_SHADER_SOURCE: &str = include_str!("imgui.frag");

fn upload_font_texture(gl: &Gl, mut fonts: imgui::FontAtlasRefMut) -> TextureHandle {
    let texture = fonts.build_rgba32_texture();

    let tex_handle = TextureHandle::new(
        gl,
        R8G8B8A8_SNORM,
        &Dimensions::Dim2d {
            width: texture.width,
            height: texture.height,
            array_layers: 1,
        },
        1,
        1,
    );

    unsafe {
        gl.TextureSubImage2D(
            tex_handle.obj,
            0,
            0,
            0,
            texture.width as i32,
            texture.height as i32,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            texture.data.as_ptr() as *const c_void,
        );
    }

    tex_handle
}

/// Renderer for dear imgui.
pub struct ImGuiRenderer {
    vao: VertexArrayHandle,
    font_tex: TextureHandle,
    program: ProgramHandle,
    matrix_loc: GLint,
}

impl ImGuiRenderer {
    /// Creates a new renderer.
    pub fn new(gl: &Gl, context: &mut imgui::Context) -> ImGuiRenderer {
        let vao = VertexArrayHandle::new(
            gl,
            &[&VertexLayout {
                elements: &[
                    VertexLayoutElement {
                        semantic: None,
                        format: Format::R32G32_SFLOAT,
                        offset: 0,
                    },
                    VertexLayoutElement {
                        semantic: None,
                        format: Format::R32G32_SFLOAT,
                        offset: 8,
                    },
                    VertexLayoutElement {
                        semantic: None,
                        format: Format::R8G8B8A8_UNORM,
                        offset: 16,
                    },
                ],
                stride: 20,
            }],
        );

        let vs = VertexShaderHandle::from_glsl(gl, IMGUI_VERTEX_SHADER_SOURCE)
            .expect("failed to compile imgui shader");
        let fs = FragmentShaderHandle::from_glsl(gl, IMGUI_FRAGMENT_SHADER_SOURCE)
            .expect("failed to compile imgui shader");
        let program = ProgramHandle::link(gl, &vs, &fs).expect("failed to link imgui program");
        let font_tex = upload_font_texture(gl, context.fonts());
        let matrix_loc =
            unsafe { gl.GetUniformLocation(program.obj, b"matrix\0".as_ptr() as *const i8) };

        ImGuiRenderer {
            vao,
            font_tex,
            program,
            matrix_loc,
        }
    }

    /// Renders the specified imgui frame
    pub fn render(&self, gl: &Gl, ui: imgui::Ui) {
        let draw_data = ui.render();
        let fb_width = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_height = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if !(fb_width > 0.0 && fb_height > 0.0) {
            return;
        }

        let left = draw_data.display_pos[0];
        let right = draw_data.display_pos[0] + draw_data.display_size[0];
        let top = draw_data.display_pos[1];
        let bottom = draw_data.display_pos[1] + draw_data.display_size[1];
        let matrix = [
            [(2.0 / (right - left)), 0.0, 0.0, 0.0],
            [0.0, (2.0 / (top - bottom)), 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [
                (right + left) / (left - right),
                (top + bottom) / (bottom - top),
                0.0,
                1.0,
            ],
        ];

        unsafe {
            gl.BindVertexArray(self.vao.obj);
            gl.Disable(gl::DEPTH_TEST);
            gl.Viewport(
                0,
                0,
                draw_data.display_size[0] as i32,
                draw_data.display_size[1] as i32,
            );
            gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
            gl.UseProgram(self.program.obj);
            gl.UniformMatrix4fv(self.matrix_loc, 1, gl::FALSE, matrix.as_ptr() as *const f32);
            gl.BindTextureUnit(0, self.font_tex.obj);
            gl.Enable(gl::BLEND);
            gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        for draw_list in draw_data.draw_lists() {
            // vertex buffer
            let vb = unsafe {
                BufferHandle::with_data(
                    gl,
                    draw_list.vtx_buffer().len() * mem::size_of::<imgui::DrawVert>(),
                    0,
                    draw_list.vtx_buffer().as_ptr() as *const c_void,
                )
            };
            // index buffer
            let ib = unsafe {
                BufferHandle::with_data(
                    gl,
                    draw_list.idx_buffer().len() * mem::size_of::<imgui::DrawIdx>(),
                    0,
                    draw_list.idx_buffer().as_ptr() as *const c_void,
                )
            };

            unsafe {
                gl.BindVertexBuffers(
                    0,
                    1,
                    &[vb.obj] as *const GLuint,
                    &[0isize] as *const GLintptr,
                    &[mem::size_of::<imgui::DrawVert>() as GLsizei] as *const _,
                );
                gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ib.obj);
            }

            for cmd in draw_list.commands() {
                match cmd {
                    imgui::DrawCmd::Elements { count, cmd_params } => unsafe {
                        gl.Scissor(
                            (cmd_params.clip_rect[0] * draw_data.framebuffer_scale[0]) as i32,
                            (cmd_params.clip_rect[1] * draw_data.framebuffer_scale[1]) as i32,
                            ((cmd_params.clip_rect[2] - cmd_params.clip_rect[0])
                                * draw_data.framebuffer_scale[0])
                                as i32,
                            ((cmd_params.clip_rect[3] - cmd_params.clip_rect[1])
                                * draw_data.framebuffer_scale[1])
                                as i32,
                        );
                        gl.DrawElementsBaseVertex(
                            gl::TRIANGLES,
                            count as i32,
                            gl::UNSIGNED_SHORT,
                            (cmd_params.idx_offset * 2) as *const c_void,
                            cmd_params.vtx_offset as i32,
                        );
                    },
                    DrawCmd::ResetRenderState => {
                        // ?
                    }
                    DrawCmd::RawCallback { callback, raw_cmd } => {
                        // TODO
                        unsafe {
                            callback(draw_list.raw(), raw_cmd);
                        }
                    }
                }
            }
        }
    }
}
