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
