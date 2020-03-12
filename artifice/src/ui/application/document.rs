use crate::document::DocumentChangeListener;
use crate::document::Document;
use crate::ui::common::platform::PlatformWindow;
use crate::ui::common::WindowEventTarget;
use crate::ui::common::WindowCtx;
use crate::ui::common::EventResult;
use crate::render::gl::api::gl;

use crate::document::DocumentId;
use crate::document::SceneId;

use winit::window::{WindowId, WindowBuilder};
use winit::event::WindowEvent;
use winit::event::ElementState;
use winit::event::MouseButton;
use std::rc::Rc;
use std::cell::Cell;
use std::cell::RefCell;
use std::path::Path;
use anyhow::Result;
use log::trace;

const CMD_MENU_FILE_OPEN: u32 = 0x101;
const CMD_MENU_FILE_EXIT: u32 = 0x100;

/// Listens to changes in a document, updates the window in return.
struct DocumentViewWrapper {
    changed_title: Option<String>,
}

impl DocumentChangeListener for DocumentViewWrapper {
    fn name_changed(&mut self, doc: &Document) {
        trace!("document name changed");
    }

    fn scene_added(&mut self, id: DocumentId, doc: &Document, scene: SceneId) {
        trace!("scene added");
    }
}

fn document_window_title_bar(doc: &Document) -> String {
    let unsaved = if doc.has_unsaved_changes() { "(*)" } else { "" };
    format!("Artifice - {} {}", doc.name, unsaved)
}

pub struct DocumentWindowHandler {
    /// The document itself
    doc: Document,
    window: PlatformWindow,
    /// Open document counter. When this reaches zero, the application should exit.
    docs_counter: Rc<Cell<usize>>,
    wrap: Rc<RefCell<DocumentViewWrapper>>,
    size: (f64, f64),
}

impl WindowEventTarget for DocumentWindowHandler {
    fn window_id(&self) -> WindowId {
        self.window.id()
    }

    fn event(&mut self, ctx: &mut WindowCtx, event: WindowEvent) -> EventResult {
        trace!("window event: {:?}", event);

        match event {
            WindowEvent::Resized(size) => {
                let size: (u32,u32) = size.into();
                self.window.resize(ctx, size);
            }
            WindowEvent::MouseInput { device_id, state, button, .. } => {
                if state == ElementState::Pressed && button == MouseButton::Right {

                    //ContextMenu::open(ctx, );

                    //ctx.add_window(window);
                }
            }
            _ => {}
        }

        Default::default()
    }

    fn paint(&mut self, ctx: &mut WindowCtx) {
        // paint here!
        {
            let gl_guard = self.window.draw_gl();
            let gl = gl_guard.functions();
            let fbo = gl_guard.framebuffer();
            unsafe {
                gl.BindFramebuffer(gl::DRAW_FRAMEBUFFER, fbo.obj);
                gl.ClearColor(0.0, 0.278, 0.671, 1.0);
                gl.Clear(gl::COLOR_BUFFER_BIT);
            }
        }

        self.window.present()
    }
}

/*pub struct DocumentWindowHandler {
    /// The document itself
    doc: Document,
    /// Open document counter. When this reaches zero, the application should exit.
    docs_counter: Rc<Cell<usize>>,
    wrap: Rc<RefCell<DocumentViewWrapper>>,
    wnd: WindowHandle,
    size: (f64, f64),
    gl_context: Option<RawContext<PossiblyCurrent>>,
    gl: Option<Gl>,
}*/

/*impl DocumentWindowHandler {
    pub fn new(
        docs_counter: Rc<Cell<usize>>,
        doc: Document,
    ) -> DocumentWindowHandler
    {
        let wrap = Rc::new(RefCell::new(DocumentViewWrapper {wnd: WindowHandle::default()}));
        DocumentWindowHandler {
            doc,
            docs_counter,
            wrap,
            wnd: WindowHandle::default(),
            size: (0.0, 0.0),
            gl_context: None,
            gl: None,
        }
    }
}*/

/*impl WinHandler for DocumentWindowHandler {
    fn connect(&mut self, handle: &WindowHandle) {
        self.wnd = handle.clone();
        let mut wrap = self.wrap.borrow_mut();
        wrap.wnd = handle.clone();
        wrap.name_changed(&self.doc);

        // build opengl context
        unsafe {
            let raw_context = ContextBuilder::new()
                .with_gl_profile(glutin::GlProfile::Core)
                .with_gl_debug_flag(true)
                .with_vsync(true)
                .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (4, 6)))
                .build_raw_context(handle.platform_handle().get_hwnd().unwrap() as *mut c_void)
                .expect("failed to create OpenGL context on window");

            let raw_context = raw_context
                .make_current()
                .expect("could not make context current");

            let gl = Gl::load_with(|symbol| {
                let ptr = raw_context.get_proc_address(symbol) as *const _;
                ptr
            });

            self.gl_context = Some(raw_context);
            self.gl = Some(gl);
        }
    }

    fn paint(&mut self, rctx: &mut RenderContext, wctx: &mut dyn WinCtx) -> bool {

        let  (width,height) = self.size;
        // OpenGL test

        unsafe {
            let ctx = self.gl_context.take().unwrap().make_current().unwrap();
            self.gl_context = Some(ctx);
            let gl = self.gl.as_ref().unwrap();
            gl.ClearColor(0.0, 1.0, 0.0, 1.0);
            gl.Clear(gl::COLOR_BUFFER_BIT);
        }



        //let (width, height) = self.size;
        //let rect = Rect::new(0.0, 0.0, width, height);
        //rctx.fill(rect, &BG_COLOR);


        let font_size = 16.0;
        let font = rctx
            .text()
            .new_font_by_name("Segoe UI", font_size)
            .build()
            .unwrap();
        let layout = rctx
            .text()
            .new_text_layout(&font, "- No signal -")
            .build()
            .unwrap();
        let w = layout.width();
        let h = font_size;
        dbg!(layout.width());
        let pos = (0.5 * (width - w), 0.5 * (height + h));

        rctx.draw_text(&layout, pos, &FG_COLOR);

        unsafe {
            self.gl_context.as_ref().unwrap().swap_buffers();
         }
        false
    }

    fn size(&mut self, width: u32, height: u32, _ctx: &mut dyn WinCtx) {
        let dpi = self.wnd.get_dpi();
        let dpi_scale = dpi as f64 / 96.0;
        let width_f = (width as f64) / dpi_scale;
        let height_f = (height as f64) / dpi_scale;
        self.size = (width_f, height_f);
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn destroy(&mut self, ctx: &mut dyn WinCtx) {}

    fn command(&mut self, id: u32, ctx: &mut dyn WinCtx) {
        match id {
            CMD_MENU_FILE_OPEN => {
                let fdo = FileDialogOptions::new().allowed_types(allowed_file_types());
                let fi = ctx.open_file_sync(fdo);
                if let Some(fi) = fi {
                    open_document_window(self.docs_counter.clone(), Some(fi.path()));
                }
            }
            CMD_MENU_FILE_EXIT => {
                error!("unimplemented");
            }
            _ => {}
        }
    }
}*/

/*fn allowed_file_types() -> Vec<FileSpec> {
    vec![FileSpec {
        name: "GLTF",
        extensions: &["gltf"]
    }]
}*/

/*fn build_document_window_menu_bar() -> Menu {
    let mut file_menu = Menu::new();
    file_menu.add_item(
        CMD_MENU_FILE_OPEN,
        "O&pen...",
        Some(&HotKey::new(SysMods::Cmd, "o")),
        true,
        false,
    );
    file_menu.add_item(
        CMD_MENU_FILE_EXIT,
        "E&xit",
        Some(&HotKey::new(SysMods::Cmd, "q")),
        true,
        false,
    );
    let mut menubar = Menu::new();
    menubar.add_dropdown(file_menu, "&File", true);
    menubar
}*/

// UiRoot
// OpenWindows
// RunLoop
// RunLoopCtx

pub fn open_document_window(
    ctx: &mut WindowCtx,
    docs_counter: Rc<Cell<usize>>,
    path: Option<&Path>,
) -> Result<()> {
    let doc = if let Some(path) = path {
        Document::from_gltf(path)?
    } else {
        Document::new()
    };

    let window =
        PlatformWindow::new(ctx, WindowBuilder::new().with_title(document_window_title_bar(&doc)), true)?;

    let handler = DocumentWindowHandler {
        doc,
        docs_counter,
        window,
        wrap: Rc::new(RefCell::new(DocumentViewWrapper {
            changed_title: None,
        })),
        size: (0.0, 0.0),
    };
    ctx.add_window(handler);

    Ok(())
}

