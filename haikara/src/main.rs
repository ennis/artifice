use anyhow::anyhow;
use glutin::{ContextBuilder, GlProfile};
use imgui::im_str;
use winit::dpi::LogicalSize;
use winit::event::Event;
use winit::event::WindowEvent;
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

use artifice_gfxbase::dimensions::Dimensions;
use artifice_gfxbase::format::Format;
use artifice_opengl::api::gl;
use artifice_opengl::api::gl::types::*;
use artifice_opengl::api::Gl;
use artifice_opengl::buffer::BufferHandle;
use artifice_opengl::framebuffer::FramebufferAttachment::Texture;
use artifice_opengl::framebuffer::{FramebufferAttachment, FramebufferHandle};
use artifice_opengl::shader::{FragmentShaderHandle, ProgramHandle, VertexShaderHandle};
use artifice_opengl::texture::TextureHandle;
use artifice_opengl::vertex_array::VertexArrayHandle;
use artifice_opengl::VertexData;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::os::raw::{c_char, c_void};
use std::path::Path;
use std::time::Instant;
use std::{fmt, mem, ptr};

use std::collections::HashMap;
use std::f32::consts::PI;
use nalgebra::{Dynamic, VecStorage, Matrix, Vector3, Matrix4, Point3, Perspective3, VectorN, MatrixN, DVector, DMatrix};
use glutin::CreationError::Window;
use std::ffi::CString;
//use artifice_opengl::draw::{Uniforms, Uniform};

mod imgui_glue;

#[repr(C)]
#[derive(Copy, Clone, VertexData)]
struct Vertex {
    pos: [f32; 3],
    norm: [f32; 3],
    tex: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, VertexData)]
struct Vertex2D {
    pos: [f32; 2],
}

const VERTEX_SHADER: &str = include_str!("main.vert");
const FRAGMENT_SHADER: &str = include_str!("main.frag");

const VISUALIZATION_VERTEX_SHADER: &str = include_str!("visualizer.vert");
const VISUALIZATION_FRAGMENT_SHADER: &str = include_str!("visualizer.frag");

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;

type DMatrixf32 = DMatrix<f32>;
type DVectorf32 = DVector<f32>;

fn optimize(gl: &Gl, out_tex: &TextureHandle, diffuse_data: &[[f32; 4]], normals_data: &[[f32; 4]], eps: f32) {
    fn packi16x3(x: f32, y: f32, z: f32) -> [i16;3] {
        let m = (i16::MAX - 1) as f32;
        [(x * m) as i16, (y * m) as i16, (z * m) as i16]
    }

    /*fn unpacki16x3(x: f32, y: f32, z: f32) -> [i16;3] {
        let m = (i16::MAX - 1) as f32;
        [(x * m) as i16, (y * m) as i16, (z * m) as i16]
    }*/

    // first, convert normals to i16x3
    let mut filtered: Vec<_> = normals_data
        .iter().zip(diffuse_data.iter())
        .filter_map(|(&[x, y, z, w], &[diff,_,_,_])| {
            if w > 0.5 {
                Some((packi16x3(x,y,z),diff))
            } else {
                None
            }
        })
        .collect();

    let mut map = HashMap::new();

    for (n, v) in filtered.iter() {
        map.insert(*n, *v);
    }

    // now flatten
    let n = map.len();
    let mut normals = Vec::with_capacity(n);
    let mut values = DVector::from_element(n, 0.0);
    let mut i = 0;
    for (n,v) in map.iter() {
        normals.push(*n);
        values[i] = *v;
        i+=1;
    }

    eprintln!("{:?}", normals);
    eprintln!("{:?}", values);

    // RBF
    fn phi(x2: f32, eps: f32) -> f32 {
        f32::exp(-eps*eps*x2)
    }

    fn norm_squared_i16x3(a: &[i16;3], b: &[i16;3]) -> f32 {
        let na = Vector3::new(a[0] as f32, a[1] as f32, a[2] as f32);
        let nb = Vector3::new(b[0] as f32, b[1] as f32, b[2] as f32);
        (na-nb).norm_squared()
    }

    // build matrix
    eprintln!("{} values", n);
    eprintln!("building RBF coefficient matrix...");
    let mut m  = DMatrixf32::identity(n,n);
    for i in 0..n { // row
        for j in 0..i { // col
            let na = Vector3::new(normals[i][0] as f32, normals[i][1] as f32, normals[i][2] as f32);
            let nb = Vector3::new(normals[j][0] as f32, normals[j][1] as f32, normals[j][2] as f32);
            let p = phi(norm_squared_i16x3(&normals[i], &normals[j]), eps);
            m[(i,j)] = p;
            m[(j,i)] = p;
        }
    }

    eprintln!("{}", m.slice((0,0),(6,6)));

    // decomp
    eprintln!("Cholesky decomp...");
    let ll = m.cholesky().expect("could not decompose the RBF matrix");

    eprintln!("Solving...");
    let s = ll.solve(&values);

    eprintln!("weights = {}", s);

    // now let's fill the normal array
    let d = WIDTH as usize;
    let mut texdata = vec![0f32; d*d*d];

    for i in 0..d {
        eprintln!("filling slice {} of {}...", i+1, d);
        for j in 0..d {
            for k in 0..d {
                let dd = (d-1) as f32;
                let nijk = packi16x3(2.0f32 * (i as f32 / dd - 0.5f32), 2.0f32 * (j as f32 / dd- 0.5f32), 2.0f32 *(k as f32 / dd- 0.5f32));
                texdata[(d-i-1)*d*d + (d-j-1)*d + (d-k-1)] =
                    s.iter().zip(normals.iter()).map(|(beta,n)| beta * phi(norm_squared_i16x3(&nijk, n), eps)).sum();
            }
        }
    }

    // upload
    unsafe {
        gl.TextureSubImage3D(out_tex.obj, 0, 0, 0, 0,
                             WIDTH as i32, WIDTH as i32, WIDTH as i32,
        gl::RED, gl::FLOAT, texdata.as_ptr() as *const _);
    }
}

/// Sets up the OpenGL debug output so that we have more information in case the interop fails.
unsafe fn init_debug_callback(gl: &Gl) {
    gl.Enable(gl::DEBUG_OUTPUT);
    gl.Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);

    if gl.DebugMessageCallback.is_loaded() {
        extern "system" fn debug_callback(
            source: GLenum,
            gltype: GLenum,
            _id: GLuint,
            severity: GLenum,
            _length: GLsizei,
            message: *const GLchar,
            _user_param: *mut c_void,
        ) {
            unsafe {
                use std::ffi::CStr;
                if severity != gl::DEBUG_SEVERITY_HIGH && severity != gl::DEBUG_SEVERITY_MEDIUM {
                    return;
                }
                let message = CStr::from_ptr(message);
                eprintln!("{:?}", message);
            }
        }
        gl.DebugMessageCallback(Some(debug_callback), ptr::null());
    }
}

fn load_obj<P: AsRef<Path> + fmt::Debug>(file_name: P) -> anyhow::Result<(Vec<Vertex>, Vec<u32>)> {
    let (models, materials) = tobj::load_obj(file_name, true)?;

    let model = models.first().ok_or(anyhow!("No model inside"))?;
    let num_vertices = model.mesh.positions.len() / 3;
    let mut vertices = Vec::with_capacity(num_vertices);

    for i in 0..num_vertices {
        vertices.push(Vertex {
            pos: [
                model.mesh.positions[i * 3],
                model.mesh.positions[i * 3 + 1],
                model.mesh.positions[i * 3 + 2],
            ],
            norm: [
                model.mesh.normals[i * 3],
                model.mesh.normals[i * 3 + 1],
                model.mesh.normals[i * 3 + 2],
            ],
            tex: [0.0, 0.0],
        });
    }

    Ok((vertices, model.mesh.indices.clone()))
}

fn get_uniform_location(gl: &Gl, prog: &ProgramHandle, name: &str) -> i32 {
    unsafe {
        let name_cstr = CString::new(name).unwrap();
        gl.GetUniformLocation(prog.obj, name_cstr.as_ptr())
    }
}

fn create_program(gl: &Gl, vert: &str, frag: &str) -> ProgramHandle {
    let vertex_shader =
        VertexShaderHandle::from_glsl(gl, vert).expect("failed to compile shader");
    let fragment_shader =
        FragmentShaderHandle::from_glsl(gl, frag).expect("failed to compile shader");
    ProgramHandle::link(gl, &vertex_shader, &fragment_shader).expect("failed to link program")
}

fn main() {
    let el = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(WIDTH, HEIGHT));
    let windowed_context = ContextBuilder::new()
        .with_gl_profile(GlProfile::Core)
        .build_windowed(wb, &el)
        .unwrap();
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    println!(
        "Pixel format of the window's GL context: {:?}",
        windowed_context.get_pixel_format()
    );

    // load opengl API
    let gl = Gl::load_with(|f| windowed_context.get_proc_address(f));

    unsafe {
        init_debug_callback(&gl);
    }

    // imgui setup
    let mut imgui_ctx = imgui::Context::create();
    let imgui_renderer = imgui_glue::ImGuiRenderer::new(&gl, &mut imgui_ctx);
    let mut platform = WinitPlatform::init(&mut imgui_ctx);
    platform.attach_window(
        imgui_ctx.io_mut(),
        &windowed_context.window(),
        HiDpiMode::Default,
    );

    // VAO setup
    let vao = VertexArrayHandle::from_vertex_data::<Vertex>(&gl);

    // Shader setup
    let prog = create_program(&gl, VERTEX_SHADER, FRAGMENT_SHADER);
    let light_mat_loc = get_uniform_location(&gl, &prog, "lightMatrix");
    let view_proj_mat_loc = get_uniform_location(&gl, &prog, "viewProjMatrix");
    let model_mat_loc = get_uniform_location(&gl, &prog, "modelMatrix");
    let show_shading_loc = get_uniform_location(&gl, &prog, "showShading");

    //let visualization_prog = create_program(&gl, VISUALIZATION_VERTEX_SHADER, VISUALIZATION_FRAGMENT_SHADER);
    //let vis_transform_loc = get_uniform_location(&gl, &visualization_prog, "transform");

    // render target setup
    let format = Format::R16G16B16A16_SFLOAT;
    let dimensions: Dimensions = (WIDTH, HEIGHT).into();
    let tex_diffuse = TextureHandle::new(&gl, format, &dimensions, 1, 1);
    let tex_normals = TextureHandle::new(&gl, format, &dimensions, 1, 1);
    let tex_depth = TextureHandle::new(&gl, Format::D32_SFLOAT, &dimensions, 1, 1);

    let framebuffer = unsafe {
        FramebufferHandle::builder()
            .color_texture(&tex_diffuse)
            .color_texture(&tex_normals)
            .depth_texture(&tex_depth)
            .build(&gl)
            .expect("failed to create framebuffer")
    };

    // result 3D texture
    let dims = Dimensions::Dim3d { width: WIDTH, height: WIDTH, depth: WIDTH };
    let interpolated_shading = TextureHandle::new(&gl, Format::R32_SFLOAT, &dims, 1, 1);
    unsafe {
        let d = WIDTH as usize;
        let mut texdata = vec![0f32; d*d*d];

        for i in 0..d {
            for j in 0..d {
                for k in 0..d {
                    texdata[i*d*d + j*d + k] = 0.5;
                }
            }
        }

        gl.TextureSubImage3D(interpolated_shading.obj, 0, 0, 0, 0,
                             WIDTH as i32, WIDTH as i32, WIDTH as i32,
                             gl::RED, gl::FLOAT, texdata.as_ptr() as *const _);
    }

    // upload mesh data
    let (vertices, indices) = load_obj("data/sphere.obj").expect("failed to load mesh");
    let num_vertices = vertices.len();
    let num_indices = indices.len();
    let buffer = unsafe {
        BufferHandle::with_data(
            &gl,
            mem::size_of::<Vertex>() * vertices.len(),
            0,
            vertices.as_ptr() as *const c_void,
        )
    };
    let indices = unsafe {
        BufferHandle::with_data(
            &gl,
            mem::size_of::<u32>() * indices.len(),
            0,
            indices.as_ptr() as *const c_void,
        )
    };

    // camera & light setup
    let light_mat: Matrix4<f32> = Matrix4::look_at_rh(
        &Point3::new(-1.0, -1.0, -1.0),
        &Point3::new(1.0, 1.0, 1.0),
        &Vector3::new(0.0, 1.0, 0.0),
    );
    let view_proj_mat: Matrix4<f32> =
        Matrix4::new_perspective(1.0f32, PI / 2.0f32, 0.1f32, 5.0f32)  *
            Matrix4::look_at_rh(
                &Point3::new(0.0, 0.0, -2.0),
                &Point3::origin(),
                &Vector3::new(0.0, 1.0, 0.0),
        );
    let model_mat: Matrix4<f32> = Matrix4::identity();

    let mut opt_eps = 0.001f32;
    let mut last_frame = Instant::now();
    let mut show_shading = false;

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::NewEvents(_) => {
                last_frame = imgui_ctx.io_mut().update_delta_time(last_frame);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == windowed_context.window().id() => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                unsafe {
                    // commands:
                    // - clear
                    // - draw
                    // - blit

                    // - draw(fbo, program, uniforms, viewport, etc.)
                    // - a lot of states tend to stay the same:
                    //      - viewport
                    //      - depth, etc.
                    // state cache?
                    // state groups?
                    // - avoid dynamic allocations (of vecs)
                    //
                    //
                    // let rs = RenderStates::new();
                    // rs.viewport(...);
                    // rs.uniform_buffers(&[...])


                    /*gl.draw(
                        DefaultFramebuffer,

                        prog,
                        mesh,

                        |u| {
                            u.uniform1i("showShading", if show_shading { 1 } else { 0 });
                            u.uniform_matrix_4f("modelMatrix", &model_mat);
                            u.uniform_matrix_4f("lightMatrix", &view_proj);
                            u.uniform_matrix_4f("viewProjMatrix", &view_proj);
                            u.textures(0, &[interpolated_shading.obj]);
                        },

                        |s| {
                            s.viewport(0,0,w,h);
                        }
                    );*/


                    gl.ClearColor(0.0, 0.0, 0.0, 0.0);
                    gl.ClearDepth(1.0);

                    if show_shading {
                        // drawing to the screen
                        gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
                        let w = windowed_context.window().inner_size();
                        gl.Viewport(0,0,w.width as i32, w.height as i32);
                    } else {
                        // draw to the fbo
                        gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, framebuffer.obj);
                        gl.Viewport(0,0,WIDTH as i32,HEIGHT as i32);
                    }
                    gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                    gl.Enable(gl::DEPTH_TEST);
                    gl.DepthFunc(gl::LESS);

                    gl.UseProgram(prog.obj);

                    gl.UniformMatrix4fv(model_mat_loc, 1, gl::FALSE, model_mat.as_ptr());
                    gl.UniformMatrix4fv(light_mat_loc, 1, gl::FALSE, light_mat.as_ptr());
                    gl.UniformMatrix4fv(view_proj_mat_loc, 1, gl::FALSE, view_proj_mat.as_ptr());
                    gl.Uniform1i(show_shading_loc, if show_shading { 1 } else { 0 });
                    gl.BindTextureUnit(0, interpolated_shading.obj);

                    gl.BindVertexArray(vao.obj);
                    gl.BindVertexBuffers(
                        0,
                        1,
                        &[buffer.obj] as *const GLuint,
                        &[0isize] as *const isize,
                        &[Vertex::LAYOUT.stride as i32] as *const i32,
                    );
                    gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, indices.obj);

                    gl.DrawElements(
                        gl::TRIANGLES,
                        num_indices as GLsizei,
                        gl::UNSIGNED_INT,
                        ptr::null(),
                    );

                    //gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, 0);
                    //gl.Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                }

                platform
                    .prepare_frame(imgui_ctx.io_mut(), windowed_context.window())
                    .expect("Failed to prepare frame");

                let ui = imgui_ctx.frame();
                imgui::Slider::new(im_str!("shape param"), 0.00001..=0.005).build(&ui, &mut opt_eps);
                ui.checkbox(im_str!("Show shading"), &mut show_shading);
                if ui.small_button(im_str!("Optimize")) {
                    // readback
                    unsafe {
                        let size = (WIDTH * HEIGHT) as usize;
                        let mut diffuse_buf: Vec<[f32; 4]> = Vec::with_capacity(size);
                        let mut normals_buf: Vec<[f32; 4]> = Vec::with_capacity(size);
                        gl.GetTextureImage(
                            tex_diffuse.obj,
                            0,
                            gl::RGBA,
                            gl::FLOAT,
                            (size * 16) as i32,
                            diffuse_buf.as_mut_ptr() as *mut c_void,
                        );
                        gl.GetTextureImage(
                            tex_normals.obj,
                            0,
                            gl::RGBA,
                            gl::FLOAT,
                            (size * 16) as i32,
                            normals_buf.as_mut_ptr() as *mut c_void,
                        );
                        diffuse_buf.set_len(size);
                        normals_buf.set_len(size);

                        optimize(&gl, &interpolated_shading, &diffuse_buf, &normals_buf, opt_eps);
                    }
                }

                imgui_renderer.render(&gl, ui);

                windowed_context.swap_buffers().unwrap();
            }
            event => {
                platform.handle_event(imgui_ctx.io_mut(), windowed_context.window(), &event);
                // step 3
                // other application-specific event handling
            }
        }
    });
}
