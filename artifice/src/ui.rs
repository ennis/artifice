//! User interface component.
use crate::render::gl::api::Gl;
use std::cell::RefCell;
use std::rc::Rc;

use druid_shell::kurbo::{Line, Rect, RoundedRect, Vec2};
use druid_shell::piet::{
    Color, FontBuilder, Piet, RenderContext, Text, TextLayout, TextLayoutBuilder,
};
use druid_shell::{
    Application, FileDialogOptions, FileSpec, HotKey, Menu, RunLoop, SysMods, WinCtx, WinHandler,
    WindowBuilder, WindowHandle,
};

use glutin::platform::windows::RawContextExt;
use glutin::{ContextBuilder, GlRequest, PossiblyCurrent, RawContext};
use std::any::Any;
use std::os::raw::c_void;

use crate::application::Component;

struct MainWindowState {
    size: (f64, f64),
    handle: WindowHandle,
    gl_context: Option<RawContext<PossiblyCurrent>>,
    gl: Option<Gl>,
}

impl Default for MainWindowState {
    fn default() -> Self {
        MainWindowState {
            size: (0.0, 0.0),
            handle: WindowHandle::default(),
            gl_context: None,
            gl: None,
        }
    }
}

const BG_COLOR: Color = Color::rgb8(0x27, 0x28, 0x22);
const FG_COLOR: Color = Color::rgb8(0xf0, 0xf0, 0xea);

const APPLICATION_CLOSE: u32 = 100;
const APPLICATION_FILE_OPEN: u32 = 101;

impl WinHandler for MainWindowState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();

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

    fn paint(&mut self, piet: &mut Piet, _ctx: &mut dyn WinCtx) -> bool {
        let (width, height) = self.size;
        let rect = Rect::new(0.0, 0.0, width, height);
        piet.fill(rect, &BG_COLOR);

        let font_size = 30.0;
        let font = piet
            .text()
            .new_font_by_name("AirbusMCDUa", font_size)
            .build()
            .unwrap();
        let layout = piet
            .text()
            .new_text_layout(&font, "- No signal -")
            .build()
            .unwrap();
        let w = layout.width();
        let h = font_size;
        dbg!(layout.width());
        let pos = (0.5 * (width - w), 0.5 * (height + h));
        piet.draw_text(&layout, pos, &FG_COLOR);
        false
    }

    fn command(&mut self, id: u32, ctx: &mut dyn WinCtx) {
        match id {
            0x100 => {
                self.handle.close();
                Application::quit();
            }
            0x101 => {
                let options = FileDialogOptions::new()
                    .show_hidden()
                    .allowed_types(vec![FileSpec::new("Artifice project files", &["atf"])]);
                let filename = ctx.open_file_sync(options);
                println!("result: {:?}", filename);
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn size(&mut self, width: u32, height: u32, _ctx: &mut dyn WinCtx) {
        let dpi = self.handle.get_dpi();
        let dpi_scale = dpi as f64 / 96.0;
        let width_f = (width as f64) / dpi_scale;
        let height_f = (height as f64) / dpi_scale;
        self.size = (width_f, height_f);
    }

    fn destroy(&mut self, _ctx: &mut dyn WinCtx) {
        Application::quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

/// The user interface component.
pub struct UserInterface {
    main_window: WindowHandle,
}

impl Component for UserInterface {}

impl UserInterface {
    /// Creates the user interface component of the application.
    ///
    /// This also opens the main window.
    pub fn new() -> Rc<RefCell<UserInterface>> {
        // --- build the main menu of the main window ---
        let mut file_menu = Menu::new();
        file_menu.add_item(
            0x100,
            "E&xit",
            Some(&HotKey::new(SysMods::Cmd, "q")),
            true,
            false,
        );
        file_menu.add_item(
            0x101,
            "O&pen",
            Some(&HotKey::new(SysMods::Cmd, "o")),
            true,
            false,
        );
        let mut menubar = Menu::new();
        menubar.add_dropdown(Menu::new(), "Application", true);
        menubar.add_dropdown(file_menu, "&File", true);

        // --- build the window itself ---
        let mut builder = WindowBuilder::new();
        builder.set_handler(Box::new(MainWindowState::default()));
        builder.set_menu(menubar);
        let main_window = builder.build().expect("could not open main window");
        main_window.show();

        Rc::new(RefCell::new(UserInterface { main_window }))
    }

    /// Enters the event loop of the user interface.
    pub fn enter_event_loop() {
        let mut run_loop = RunLoop::new();
        run_loop.run();
    }
}
