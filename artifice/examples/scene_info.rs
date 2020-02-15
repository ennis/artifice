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
    let ui = UserInterface::new();
    app.add_component(ui.clone());

    UserInterface::enter_event_loop();
}
