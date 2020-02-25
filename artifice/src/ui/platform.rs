//! Platform-specific initialization
use crate::render::gl::api::gl::types::*;
use crate::render::gl::api::Gl;
use crate::render::gl::api::Wgl;
use crate::render::gl::api::*;
use crate::ui::RunLoopCtx;
use anyhow::Result;
use anyhow::{Context, Error};
use glutin::platform::windows::RawContextExt;
use glutin::{ContextBuilder, GlRequest, PossiblyCurrent, RawContext};
use log::{error, info, trace};
use std::os::raw::c_void;
use std::rc::Rc;
use std::{error, fmt, ptr};
use winapi::_core::fmt::Formatter;
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgiformat::*;
use winapi::shared::minwindef::HINSTANCE;
use winapi::shared::windef::HWND;
use winapi::shared::winerror::SUCCEEDED;
use winapi::um::d2d1::*;
use winapi::um::d3d11::*;
use winapi::um::d3d11::{D3D11CreateDevice, D3D11_SDK_VERSION};
use winapi::um::d3dcommon::*;
use winapi::um::dcommon::*;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winnt::HRESULT;
use winapi::{
    shared::{dxgi1_2, dxgiformat, dxgitype, minwindef, winerror},
    um::{d3d11, d3dcommon, libloaderapi, wingdi, winuser},
};
use winit::platform::windows::WindowExtWindows;
use winit::window::{Window, WindowBuilder, WindowId};
use wio::com::ComPtr;

use com_wrapper::ComWrapper;
use direct2d::brush::SolidColorBrush;
use direct2d::enums::DrawTextOptions;
use direct3d11::enums::BindFlags;
use direct3d11::enums::Usage;
use directwrite::{TextFormat, TextLayout};
use dxgi::enums::*;
use dxgi::enums::{PresentFlags, SwapChainFlags};
use std::cell::RefCell;

#[derive(Copy, Clone)]
pub struct HResultError(pub HRESULT);

impl fmt::Debug for HResultError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for HResultError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[HRESULT {:08X}]", self.0)
    }
}

impl error::Error for HResultError {}

fn check_hr(hr: HRESULT) -> Result<HRESULT, HResultError> {
    if (!SUCCEEDED(hr)) {
        Err(HResultError(hr))
    } else {
        Ok(hr)
    }
}

unsafe fn init_debug_callback(gl: &Gl) {
    gl.Enable(gl::DEBUG_OUTPUT);
    gl.Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
    use std::os::raw::c_void;
    if gl.DebugMessageCallback.is_loaded() {
        extern "system" fn debug_callback(
            source: GLenum,
            gltype: GLenum,
            id: GLuint,
            severity: GLenum,
            length: GLsizei,
            message: *const GLchar,
            _userParam: *mut c_void,
        ) {
            unsafe {
                use std::ffi::CStr;
                let message = CStr::from_ptr(message);
                eprintln!("{:?}", message);
                match source {
                    gl::DEBUG_SOURCE_API => eprintln!("Source: API"),
                    gl::DEBUG_SOURCE_WINDOW_SYSTEM => eprintln!("Source: Window System"),
                    gl::DEBUG_SOURCE_SHADER_COMPILER => eprintln!("Source: Shader Compiler"),
                    gl::DEBUG_SOURCE_THIRD_PARTY => eprintln!("Source: Third Party"),
                    gl::DEBUG_SOURCE_APPLICATION => eprintln!("Source: Application"),
                    gl::DEBUG_SOURCE_OTHER => eprintln!("Source: Other"),
                    _ => (),
                }

                match gltype {
                    gl::DEBUG_TYPE_ERROR => eprintln!("Type: Error"),
                    gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR => eprintln!("Type: Deprecated Behaviour"),
                    gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => eprintln!("Type: Undefined Behaviour"),
                    gl::DEBUG_TYPE_PORTABILITY => eprintln!("Type: Portability"),
                    gl::DEBUG_TYPE_PERFORMANCE => eprintln!("Type: Performance"),
                    gl::DEBUG_TYPE_MARKER => eprintln!("Type: Marker"),
                    gl::DEBUG_TYPE_PUSH_GROUP => eprintln!("Type: Push Group"),
                    gl::DEBUG_TYPE_POP_GROUP => eprintln!("Type: Pop Group"),
                    gl::DEBUG_TYPE_OTHER => eprintln!("Type: Other"),
                    _ => (),
                }

                match severity {
                    gl::DEBUG_SEVERITY_HIGH => eprintln!("Severity: high"),
                    gl::DEBUG_SEVERITY_MEDIUM => eprintln!("Severity: medium"),
                    gl::DEBUG_SEVERITY_LOW => eprintln!("Severity: low"),
                    gl::DEBUG_SEVERITY_NOTIFICATION => eprintln!("Severity: notification"),
                    _ => (),
                }
                panic!();
            }
        }
        gl.DebugMessageCallback(Some(debug_callback), 0 as *mut _);
    }
}

pub struct Platform(Rc<PlatformState>);

pub struct PlatformState {
    d3d11_device: direct3d11::Device,
    d3d11_device_context: direct3d11::DeviceContext,
    dxgi_factory: dxgi::factory::Factory2,
    dwrite_factory: directwrite::Factory,
    d2d_factory: direct2d::Factory1,
    d2d_device: direct2d::Device,
    d2d_context: RefCell<direct2d::DeviceContext>,
}

impl Platform {
    pub unsafe fn init() -> Result<Platform> {
        use direct3d11::enums::*;

        let (feature_level, mut d3d11_device, mut d3d11_device_context) =
            direct3d11::Device::create()
                .with_flags(CreateDeviceFlags::BGRA_SUPPORT | CreateDeviceFlags::DEBUG)
                .with_driver_type(DriverType::Hardware)
                .build()?;
        let dxgi_factory: dxgi::factory::Factory2 = dxgi::factory::create()?;
        let dwrite_factory = directwrite::Factory::new()?;
        let d2d_factory = direct2d::Factory1::new()?;

        // Create the D2D Device and Context
        let mut d2d_device = direct2d::Device::create(&d2d_factory, &d3d11_device.as_dxgi())?;
        let mut d2d_context = RefCell::new(direct2d::DeviceContext::create(&d2d_device)?);

        /*let mut dxgi_debug = {
            let mut dxgi_debug: *mut IDXGIDebug1 = ptr::null_mut();
            check_hr(dxgi::DXGIGetDebugInterface1(0, &))?;
        };*/

        Ok(Platform(Rc::new(PlatformState {
            d3d11_device,
            d3d11_device_context,
            dxgi_factory,
            dwrite_factory,
            d2d_factory,
            d2d_device,
            d2d_context,
        })))
    }
}

struct DxGlInterop {
    gl: Gl,
    wgl: Wgl,
    /// Interop device handle
    device: wgl::types::HANDLE,
    /// Staging texture
    staging: Option<direct3d11::Texture2D>,
    /// Interop handle for the OpenGL drawing target.
    /// If `staging_d3d11` is not None, then this is a handle to the staging texture, otherwise
    /// it's a handle to the true backbuffer.
    target: wgl::types::HANDLE,
    renderbuffer: GLuint,
    fbo: GLuint,
}

/// Contains resources that should be re-created when the swap chain of a window changes
/// (e.g. on resize).
struct SwapChainResources {
    backbuffer: direct3d11::Texture2D,
    interop: Option<DxGlInterop>,
}

impl SwapChainResources {
    unsafe fn new(
        swap_chain: &dxgi::swap_chain::SwapChain1,
        _device: &direct3d11::Device,
        width: u32,
        height: u32,
    ) -> Result<SwapChainResources> {
        let buffer = swap_chain
            .buffer(0)
            .context("failed in IDXGISwapChain1::GetBuffer")?;
        Ok(SwapChainResources {
            backbuffer: buffer,
            interop: None,
        })
    }

    unsafe fn with_gl_interop(
        swap_chain: &dxgi::swap_chain::SwapChain1,
        device: &direct3d11::Device,
        gl: Gl,
        wgl: Wgl,
        width: u32,
        height: u32,
        use_staging_texture: bool,
    ) -> Result<SwapChainResources> {
        let mut res = Self::new(swap_chain, device, width, height)?;

        let interop_device = wgl.DXOpenDeviceNV(device.get_raw() as *mut _);
        if interop_device.is_null() {
            return Err(anyhow::Error::msg("could not create OpenGL-DX interop"));
        }

        let mut renderbuffer = 0;
        gl.GenRenderbuffers(1, &mut renderbuffer);

        let (staging, interop_target) = if use_staging_texture {
            // use staging texture because directly sharing the swap chain buffer when using FLIP_*
            // swap effects seems to cause problems.
            let staging = direct3d11::Texture2D::create(device)
                .with_format(Format::R8G8B8A8Unorm)
                .with_size(width, height)
                .with_bind_flags(BindFlags::RENDER_TARGET)
                .with_mip_levels(1)
                .with_usage(Usage::Default)
                .build()?;

            let interop_staging = wgl.DXRegisterObjectNV(
                interop_device,
                staging.get_raw() as *mut _,
                renderbuffer,
                gl::RENDERBUFFER,
                wgl::ACCESS_READ_WRITE_NV,
            );
            (Some(staging), interop_staging)
        } else {
            // directly share the swap chain buffer (this may cause problems with FLIP_* swap effects)
            let interop_backbuffer = wgl.DXRegisterObjectNV(
                interop_device,
                res.backbuffer.get_raw() as *mut _,
                renderbuffer,
                gl::RENDERBUFFER,
                wgl::ACCESS_READ_WRITE_NV,
            );
            (None, interop_backbuffer)
        };

        if interop_target.is_null() {
            // failed to register object, but we still created a renderbuffer: delete it.
            gl.DeleteRenderbuffers(1, &mut renderbuffer);
            return Err(Error::msg("wglDXRegisterObjectNV error"));
        }

        // create a framebuffer that points to the swap chain buffer
        let mut fbo = 0;
        gl.GenFramebuffers(1, &mut fbo);
        gl.BindFramebuffer(gl::FRAMEBUFFER, fbo);
        gl.FramebufferRenderbuffer(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::RENDERBUFFER,
            renderbuffer,
        );
        //gl.CreateFramebuffers(1, &mut fbo);
        //gl.NamedFramebufferRenderbuffer(fbo, gl::COLOR_ATTACHMENT0, gl::RENDERBUFFER, renderbuffer);

        let fb_status = gl.CheckNamedFramebufferStatus(fbo, gl::DRAW_FRAMEBUFFER);
        if fb_status != gl::FRAMEBUFFER_COMPLETE {
            // don't forget to release the GL resources still lying around. Those don't follow RAII
            // contrary to ComPtr<> wrapped resources.
            gl.DeleteFramebuffers(1, &mut fbo);
            gl.DeleteRenderbuffers(1, &mut renderbuffer);
            wgl.DXUnregisterObjectNV(interop_device, interop_target);
            return Err(Error::msg(format!(
                "could not create window framebuffer: CheckNamedFramebufferStatus returned {}",
                fb_status
            )));
        }

        res.interop = Some(DxGlInterop {
            gl,
            wgl,
            device: interop_device,
            staging,
            target: interop_target,
            renderbuffer,
            fbo,
        });

        Ok(res)
    }
}

fn check_win32_last_error(returned: i32, function: &str) {
    unsafe {
        if returned == 0 {
            let err = GetLastError();
            panic!("{} failed, GetLastError={:08x}", function, err);
        }
    }
}

impl Drop for SwapChainResources {
    fn drop(&mut self) {
        if let Some(ref mut interop) = self.interop {
            let gl = &interop.gl;
            let wgl = &interop.wgl;

            unsafe {
                gl.DeleteFramebuffers(1, &mut interop.fbo);
                gl.DeleteRenderbuffers(1, &mut interop.renderbuffer);
                check_win32_last_error(
                    wgl.DXUnregisterObjectNV(interop.device, interop.target),
                    "wglDXUnregisterObjectNV",
                );
                check_win32_last_error(wgl.DXCloseDeviceNV(interop.device), "wglDXCloseDeviceNV");
            }
        }
    }
}

pub struct GlState {
    context: RawContext<PossiblyCurrent>,
    gl: Gl,
    wgl: Wgl,
}

pub struct PlatformWindow {
    // we don't really need it to  have a shared ref here, but
    // this way we can avoid passing RunLoopCtx everywhere (which contains the platform state).
    shared: Rc<PlatformState>,
    window: Window,
    hwnd: HWND,
    hinstance: HINSTANCE,
    swap_chain: dxgi::swap_chain::SwapChain1,
    swap_res: Option<SwapChainResources>,
    gl: Option<GlState>,
    interop_needs_staging: bool,
}

const SWAP_CHAIN_BUFFERS: u32 = 2;
const USE_INTEROP_STAGING_TEXTURE: bool = false;

/// Given a window handle, creates some stuff required to draw 2D and 3D on the screen.
impl PlatformWindow {
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn resize(&mut self, ctx: &RunLoopCtx, (width, height): (u32, u32)) -> Result<()> {
        trace!("resizing swap chain: {}x{}", width, height);

        // signal the GL context as well if we have one
        if let Some(ref mut gl) = self.gl {
            gl.context.resize((width, height).into());
        }

        unsafe {
            // explicitly release swap-chain dependent resources
            self.swap_res = None;

            // resize the swap chain
            let err = self
                .swap_chain
                .resize_buffers()
                .dimensions(width, height)
                .finish();

            if let Err(err) = err {
                // it fails sometimes...
                error!("IDXGISwapChain1::ResizeBuffers failed: {}", err);
                return Ok(());
            }

            // re-create all resources that depend on the swap chain
            self.swap_res = Some(if let Some(ref mut gl) = self.gl {
                SwapChainResources::with_gl_interop(
                    &self.swap_chain,
                    &self.shared.d3d11_device,
                    gl.gl.clone(),
                    gl.wgl.clone(),
                    width,
                    height,
                    self.interop_needs_staging,
                )?
            } else {
                SwapChainResources::new(&self.swap_chain, &self.shared.d3d11_device, width, height)?
            });
        }

        Ok(())
    }

    pub fn new(ctx: &RunLoopCtx, builder: WindowBuilder, with_gl: bool) -> Result<PlatformWindow> {
        // We want to be able to render 3D stuff with OpenGL, and still be able to use
        // D3D11/Direct2D/DirectWrite.
        // To do so, we use a DXGI swap chain to manage presenting. Then, using WGL_NV_DX_interop2,
        // we register the buffers of the swap chain as a renderbuffer in GL so we can use both
        // on the same render target.
        unsafe {
            // first, build the window using the provided builder
            let window = builder.build(ctx.event_loop())?;

            let dxgi_factory = &ctx.platform.0.dxgi_factory;
            let d3d11_device = &ctx.platform.0.d3d11_device;

            // create a DXGI swap chain for the window
            let hinstance: HINSTANCE = window.hinstance() as HINSTANCE;
            let hwnd: HWND = window.hwnd() as HWND;
            let (width, height): (u32, u32) = window.inner_size().into();

            //let swap_effect = if with_gl {
            //
            //   SwapEffect::Discard
            //} else {
            //    SwapEffect::FlipDiscard
            //};
            let swap_effect = SwapEffect::Discard;

            // OpenGL interop does not work well with FLIP_* swap effects
            // (generates a "D3D11 Device Lost" error during resizing after a while).
            // In those cases, draw on a staging texture, and then copy to the backbuffer.
            let interop_needs_staging = match swap_effect {
                SwapEffect::FlipSequential | SwapEffect::FlipDiscard => true,
                _ => false,
            };

            if interop_needs_staging && with_gl {
                info!("FLIP_DISCARD or FLIP_SEQUENTIAL swap chains with OpenGL interop may cause crashes. \
                 Will allocate a staging target to work around this issue.");
            }

            let swap_chain =
                dxgi::swap_chain::SwapChain1::create_hwnd(dxgi_factory, &d3d11_device.as_dxgi())
                    .with_flags(SwapChainFlags::NONE)
                    .with_swap_effect(swap_effect)
                    .with_format(Format::R8G8B8A8Unorm)
                    .with_buffer_count(SWAP_CHAIN_BUFFERS)
                    .with_scaling(Scaling::Stretch)
                    .with_alpha_mode(AlphaMode::Unspecified)
                    .with_buffer_usage(UsageFlags::RENDER_TARGET_OUTPUT)
                    .with_hwnd(hwnd)
                    .build()?;

            // Create the OpenGL context

            let (swap_res, gl) = if with_gl {
                trace!("creating OpenGL context");
                let context = ContextBuilder::new()
                    .with_gl_profile(glutin::GlProfile::Core)
                    .with_gl_debug_flag(true)
                    .with_vsync(true)
                    .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (4, 6)))
                    .build_raw_context(hwnd as *mut c_void)
                    .expect("failed to create OpenGL context on window");
                let context = context
                    .make_current()
                    .expect("could not make context current");
                // load GL functions
                let loader = |symbol| {
                    let ptr = context.get_proc_address(symbol) as *const _;
                    ptr
                };
                let gl = Gl::load_with(loader);
                let wgl = Wgl::load_with(loader);
                // set up a debug callback so we have a clue of what's going wrong
                init_debug_callback(&gl);
                // first-time initialization of the swap chain resources, with GL interop enabled
                let swap_res = SwapChainResources::with_gl_interop(
                    &swap_chain,
                    &d3d11_device,
                    gl.clone(),
                    wgl.clone(),
                    width,
                    height,
                    interop_needs_staging,
                )?;

                let gl = GlState { context, gl, wgl };

                (swap_res, Some(gl))
            } else {
                // no OpenGL requested for this window
                let swap_res = SwapChainResources::new(&swap_chain, &d3d11_device, width, height)?;
                (swap_res, None)
            };

            let mut pw = PlatformWindow {
                shared: ctx.platform.0.clone(),
                window,
                hwnd,
                hinstance,
                swap_chain,
                swap_res: Some(swap_res),
                gl,
                interop_needs_staging,
            };

            // create initial swap chain dependent resources
            Ok(pw)
        }
    }

    pub fn draw_gl<R>(&mut self, f: impl FnOnce(&Gl, GLuint) -> R) -> Result<R> {
        // TODO type safety (PlatformWindow, PlatformGLWindow)
        // not a priority though
        let gl_state = self
            .gl
            .as_mut()
            .expect("draw_gl called but a context was not requested");
        let swap_res = self
            .swap_res
            .as_mut()
            .expect("draw_gl called but the swap chain is not initialized");
        let interop = swap_res
            .interop
            .as_mut()
            .expect("DX-GL interop not initialized");

        unsafe {
            let gl = &gl_state.gl;
            let wgl = &gl_state.wgl;
            // signals to the interop device that OpenGL is going to use the resource specified by the
            // given interop handle.
            wgl.DXLockObjectsNV(interop.device, 1, &mut interop.target);
            // call provided closure
            let r = f(&gl, interop.fbo);
            // finished using the resource
            wgl.DXUnlockObjectsNV(interop.device, 1, &mut interop.target);

            if let Some(ref staging_d3d11) = interop.staging {
                // copy staging tex to actual backbuffer
                let ctx = &self.shared.d3d11_device_context;
                let backbuffer = {
                    let ptr = ComPtr::from_raw(swap_res.backbuffer.get_raw());
                    let p2 = ptr.cast::<ID3D11Resource>().unwrap();
                    ptr.into_raw();
                    p2
                };
                let staging = {
                    let ptr = ComPtr::from_raw(staging_d3d11.get_raw());
                    let p2 = ptr.cast::<ID3D11Resource>().unwrap();
                    ptr.into_raw();
                    p2
                };
                (&*ctx.get_raw()).CopyResource(backbuffer.as_raw(), staging.as_raw());
            }

            self.test_d2d();
            self.swap_chain
                .present(1, PresentFlags::NONE)
                .expect("present failed");
            Ok(r)
        }
    }

    unsafe fn test_d2d(&mut self) -> Result<()> {
        let d2d = &self.shared.d2d_factory;
        let dwrite = &self.shared.dwrite_factory;
        //let mut context = self.shared.d2d_context.borrow_mut();
        let mut swap_res = self.swap_res.as_mut().unwrap();
        let dxgi_buffer = swap_res.backbuffer.as_dxgi();

        let dpi = 0.0;
        let props = D2D1_RENDER_TARGET_PROPERTIES {
            _type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_R8G8B8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_IGNORE,
            },
            dpiX: dpi,
            dpiY: dpi,
            usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
        };

        const DPI: f32 = 1.0;

        let mut render_target = {
            let mut render_target: *mut ID2D1RenderTarget = ptr::null_mut();
            let res = (*d2d.get_raw()).CreateDxgiSurfaceRenderTarget(
                dxgi_buffer.get_raw(),
                &props,
                &mut render_target,
            );
            direct2d::render_target::RenderTarget::from_raw(render_target)
        };

        // Get the Segoe UI font
        let font = TextFormat::create(&dwrite)
            .with_family("Segoe UI")
            .with_size(26.0)
            .build()
            .unwrap();

        // Lay out our testing text, which contains an emoji
        let text = TextLayout::create(&dwrite)
            .with_str("Testing testing! 🦆🦆🦆🦆🦆")
            .with_format(&font)
            .with_size(640.0 as f32 / DPI - 30.0, 480.0 as f32 / DPI - 30.0)
            .build()
            .unwrap();

        // Black brush for the main text
        let fg_brush = SolidColorBrush::create(&render_target)
            .with_color(0x00_00_00)
            .build()
            .unwrap();
        let bg_brush = SolidColorBrush::create(&render_target)
            .with_color(0xFF_7F_7F)
            .build()
            .unwrap();

        println!("fg: {:?}", fg_brush.color());
        println!("bg: {:?}", bg_brush.color());

        // Start drawing to the texture
        render_target.set_dpi(96.0 * DPI, 96.0 * DPI);
        render_target.begin_draw();

        // Make the background white
        //render_target.clear(0xFF_FF_FFu32);

        let rect = [10.5f32, 10.5, 190.5, 90.5];
        render_target.fill_rectangle(rect, &bg_brush);
        render_target.draw_rectangle(rect, &fg_brush, 1.0, None);

        // Draw the text
        render_target.draw_text_layout(
            (15.0, 15.0),
            &text,
            &fg_brush,
            DrawTextOptions::ENABLE_COLOR_FONT,
        );

        // Finish
        render_target.end_draw().unwrap();

        Ok(())
    }
}
