use artifice::application::Application;
use std::any::Any;

use artifice::document::Documents;
use artifice::ui::UserInterface;
use winit::window::WindowBuilder;
use winit::event_loop::EventLoop;
use winit::event_loop::ControlFlow;
use winit::event::{Event, WindowEvent};

fn main() {
    pretty_env_logger::init();

    let mut app = Application::new();

    // user interface
    let mut ui = UserInterface::new(&app);
    ui.new_document();
    ui.run()
}
