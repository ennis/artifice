use artifice::application::Application;
use druid_shell::{HotKey, Menu, SysMods, WindowBuilder};
use std::any::Any;

use artifice::document::Documents;
use artifice::ui::UserInterface;

fn main() {
    let mut app = Application::new();

    // document model
    let docs = Documents::new(&app);
    app.add_component(docs.clone());

    // user interface
    let ui = UserInterface::new(&app);
    app.add_component(ui.clone());

    // create an empty document (this will automatically open a window)
    docs.borrow_mut().new_document();

    UserInterface::enter_event_loop();
}
