//! User interface component.
use crate::application::{Application, Component};
use crate::document::{
    Document, DocumentChangeListener, DocumentId, Documents, OpenDocumentsChanges, OpenDocumentsChangeListener,
};
use druid_shell::{
    FileDialogOptions, FileSpec, HotKey, Menu, RunLoop, SysMods, WinCtx, WinHandler, WindowBuilder,
    WindowHandle,
};
use kyute::paint::RenderContext;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::os::raw::c_void;
use std::rc::Rc;

pub mod document;
use document::DocumentWindowHandler;

/// The user interface component.
pub struct UserInterface {
    docs: Rc<RefCell<Documents>>,
    windows: Vec<WindowHandle>,
    ndocs: Rc<Cell<usize>>,
}

impl Component for UserInterface {}

impl OpenDocumentsChangeListener for UserInterface {
    fn document_opened(&mut self, id: DocumentId, doc: &Document) {
        eprintln!("UserInterface::document_opened");
        // open a document window
        let mut builder = WindowBuilder::new();
        builder.set_handler(Box::new(DocumentWindowHandler::new(
            self.docs.clone(),
            self.ndocs.clone(),
            id,
        )));
        builder.set_title("TODO: document name");
        let handle = builder.build().expect("could not create document window");
        handle.show();
        self.windows.push(handle);
    }

    fn document_closed(&mut self, id: DocumentId, doc: &Document) {
        // TODO (close window?)
    }
}

impl UserInterface {
    /// Creates the user interface component of the application.
    pub fn new(app: &Application) -> Rc<RefCell<UserInterface>> {
        let docs = app
            .component::<Documents>()
            .expect("could not get the documents component");
        let windows = Vec::new();
        let ndocs = Rc::new(Cell::new(0));
        let ui = Rc::new(RefCell::new(UserInterface {
            docs: docs.clone(),
            windows,
            ndocs,
        }));
        OpenDocumentsChanges::listen(app.message_bus(), ui.clone());
        ui
    }

    /// Enters the event loop of the user interface.
    pub fn enter_event_loop() {
        let mut run_loop = RunLoop::new();
        run_loop.run();
    }
}
