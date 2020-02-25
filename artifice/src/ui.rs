//! User interface component.
use crate::application::{Application, Component};
use crate::document::{Document, DocumentId, Documents};
use log::trace;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod platform;
pub mod document;

use platform::Platform;
use document::open_document_window;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::WindowId;

/// A wrapper around a window that receives events.
pub trait WindowEventTarget {
    fn window_id(&self) -> WindowId;
    fn event(&mut self, ctx: &mut RunLoopCtx, event: WindowEvent);
    fn paint(&mut self, ctx: &mut RunLoopCtx);
}

/// Context passed to functions that create new windows
/// so that they can be registered with the event loop.
pub struct RunLoopCtx<'a> {
    elwt: &'a EventLoopWindowTarget<()>,
    /// Newly-created windows
    new_windows: Vec<Box<dyn WindowEventTarget>>,
    /// platform-specific application state
    platform: &'a Platform,
}

impl<'a> RunLoopCtx<'a> {
    fn new(elwt: &'a EventLoopWindowTarget<()>, platform: &'a Platform) -> RunLoopCtx<'a> {
        RunLoopCtx {
            elwt,
            new_windows: Vec::new(),
            platform
        }
    }

    pub fn event_loop(&self) -> &EventLoopWindowTarget<()> {
        self.elwt
    }

    pub fn register_window(&mut self, w: impl WindowEventTarget + 'static) {
        self.new_windows.push(Box::new(w));
    }
}

/// The user interface component.
pub struct UserInterface {
    event_loop: EventLoop<()>,
    docs_counter: Rc<Cell<usize>>,
    early_windows: Vec<Box<dyn WindowEventTarget>>,
    platform: Platform,
}

impl Component for UserInterface {}

impl UserInterface {
    /// Creates the user interface component of the application.
    pub fn new(app: &Application) -> UserInterface {
        let docs_counter = Rc::new(Cell::new(0));
        let event_loop = EventLoop::new();
        let platform = unsafe { platform::Platform::init().expect("failed to initialize platform state") };
        let ui = UserInterface {
            event_loop,
            docs_counter,
            early_windows: Vec::new(),
            platform
        };
        ui
    }

    /// Creates a new document.
    ///
    /// This is meant to be called before the event loop starts.
    pub fn new_document(&mut self) {
        trace!("creating a new document");
        let mut nw = {
            let mut ctx = RunLoopCtx::new(&self.event_loop, &self.platform);
            open_document_window(&mut ctx, self.docs_counter.clone(), None)
                .expect("failed to create new document window");
            ctx.new_windows
        };
        self.early_windows
            .append(&mut nw);
    }

    /// Enters the event loop of the user interface.
    pub fn run(self) -> ! {
        let mut windows = self.early_windows;
        let mut platform = self.platform;

        self.event_loop.run(move |event, elwt, control_flow| {
            *control_flow = ControlFlow::Wait;

            let mut ctx = RunLoopCtx::new(elwt, &platform);

            //
            match event {
                Event::WindowEvent { window_id, event } => {
                    // the event loop receives events for all windows
                    // find which window we should forward the event to
                    windows
                        .iter_mut()
                        .find(|w| w.window_id() == window_id)
                        .map(|w| w.event(&mut ctx, event));
                },
                Event::RedrawRequested(window_id) => {
                    windows
                        .iter_mut()
                        .find(|w| w.window_id() == window_id)
                        .map(|w| w.paint(&mut ctx));
                }
                _ => (),
            }

            windows.append(&mut ctx.new_windows);
        })
    }
}
