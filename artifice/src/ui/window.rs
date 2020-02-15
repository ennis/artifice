

impl WinHandler for WindowState {
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

        let font_size = 16.0;
        let font = piet
            .text()
            .new_font_by_name("Segoe UI", font_size)
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

    fn command(&mut self, id: u32, ctx: &mut dyn WinCtx) {}

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

pub struct Window(WindowHandle);

impl Window {
    pub fn new(mut builder: WindowBuilder) -> Window {
        builder.set_handler(Box::new(WindowState::default()));
        let handle = builder.build().expect("could not open window");
        Window(handle)
    }

    pub fn show(&self) {
        self.0.show();
    }
}