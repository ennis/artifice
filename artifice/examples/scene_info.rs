use artifice::application::Application;
use std::any::Any;

use artifice::document::Documents;
use std::cell::Cell;
use std::rc::Rc;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::window::WindowBuilder;


// opening a document window:
// - create a new document / load document
// - create window
// - window.set_document(id)
// - register it to the event loop
// - return
//

fn main() {

    // 1. init platform
    // 2. create a winit event loop
    // 3. create a platform window
    // 4. enter event loop

}