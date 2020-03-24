use artifice::application::Application;
use std::any::Any;

use artifice::document::Documents;
use artifice::ui::common::MainEventLoop;
use artifice::ui::document::open_document_window;
use std::cell::Cell;
use std::rc::Rc;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::window::WindowBuilder;

fn main() {
    pretty_env_logger::init();

    let mut app = Application::new();

    let mut events = MainEventLoop::new();
    let mut open_docs_counter = Rc::new(Cell::new(0));
    events.with_window_ctx(move |ctx| {
        open_document_window(ctx, open_docs_counter.clone(), None);
    });

    // Dispatcher: closure that receives and processes actions of type A
    // - can have multiple dispatchers?

    // VS
    //



    events.run();
}
