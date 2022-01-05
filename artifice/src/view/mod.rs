use crate::model::{Document, ModelPath, Node};
use kyute::{
    composable,
    shell::winit::window::WindowBuilder,
    widget::{Action, Axis, Button, Flex, Menu, MenuItem, Shortcut, Text},
    Cache, Key, WidgetPod, Window,
};
use rusqlite::Connection;
use std::sync::Arc;

/// Node view.
#[composable]
pub fn node_item(#[uncached] document: &mut Document, node: &Node) -> WidgetPod {
    let delete_button = Button::new("Delete".to_string());
    if delete_button.clicked() {
        eprintln!("delete node clicked {:?}", node.base.path);
        document.delete_node(node);
    }
    Flex::new(
        Axis::Horizontal,
        vec![
            Text::new(format!("{}({})", node.base.path.to_string(), node.base.id)),
            delete_button,
        ],
    )
}

/// Root document view.
#[composable]
pub fn document_window_contents(#[uncached] document: &mut Document) -> WidgetPod {
    eprintln!("document_window_contents");

    let document_model = document.model().clone();

    let flex_items = {
        // Root nodes
        let mut node_views: Vec<WidgetPod> = Vec::new();
        for (_name, node) in document_model.root.children.iter() {
            Cache::scoped(node.base.id as usize, || {
                node_views.push(node_item(document, node))
            })
        }

        // "Add Node" button
        let add_node_button = Button::new("Add Node".to_string());

        if add_node_button.clicked() {
            eprintln!("add node clicked");
            let name = document_model.root.make_unique_child_name("node");
            document.create_node(ModelPath::root().join(name));
        }

        let mut flex_items = Vec::new();
        flex_items.extend(node_views);
        flex_items.push(add_node_button);
        flex_items
    };

    // enclosing window
    Flex::new(Axis::Vertical, flex_items)
}

/// Main menu bar.
#[composable]
pub fn main_menu_bar(#[uncached] document: &mut Document) -> Menu {
    let file_new = Action::with_shortcut(Shortcut::new(
        kyute::event::Modifiers::CONTROL,
        kyute::event::Key::Character("N".to_string()),
    ));
    let file_open = Action::with_shortcut(Shortcut::new(
        kyute::event::Modifiers::CONTROL,
        kyute::event::Key::Character("O".to_string()),
    ));
    let file_save = Action::with_shortcut(Shortcut::new(
        kyute::event::Modifiers::CONTROL,
        kyute::event::Key::Character("S".to_string()),
    ));
    let file_save_as = Action::with_shortcut(Shortcut::new(
        kyute::event::Modifiers::CONTROL | kyute::event::Modifiers::SHIFT,
        kyute::event::Key::Character("S".to_string()),
    ));
    let file_quit = Action::with_shortcut(Shortcut::new(
        kyute::event::Modifiers::ALT,
        kyute::event::Key::F4,
    ));

    let edit_undo = Action::with_shortcut(Shortcut::new(
        kyute::event::Modifiers::CONTROL,
        kyute::event::Key::Character("Z".to_string()),
    ));
    let edit_redo = Action::with_shortcut(Shortcut::new(
        kyute::event::Modifiers::CONTROL,
        kyute::event::Key::Character("Y".to_string()),
    ));

    if file_new.triggered() {
        tracing::warn!("File>New unimplemented");
    }
    if file_open.triggered() {
        tracing::warn!("File>Open unimplemented");
    }
    if file_save.triggered() {
        tracing::warn!("File>Save unimplemented");
    }
    if file_save_as.triggered() {
        tracing::warn!("File>Save As unimplemented");
    }
    if file_quit.triggered() {
        tracing::warn!("File>Quit unimplemented");
    }

    if edit_undo.triggered() {
        tracing::warn!("Edit>Undo unimplemented");
    }
    if edit_redo.triggered() {
        tracing::warn!("Edit>Redo unimplemented");
    }

    let mut file_menu = Menu::new(vec![
        MenuItem::new("New", file_new),
        MenuItem::new("Open...", file_open),
        MenuItem::new("Save", file_save),
        MenuItem::new("Save as...", file_save_as),
        MenuItem::separator(),
        MenuItem::new("Quit", file_quit),
    ]);
    let mut edit_menu = Menu::new(vec![
        MenuItem::new("Undo", edit_undo),
        MenuItem::new("Redo", edit_redo),
    ]);
    let menu_bar = Menu::new(vec![
        MenuItem::submenu("File", file_menu),
        MenuItem::submenu("Edit", edit_menu),
    ]);

    menu_bar
}

/// Native window displaying a document.
#[composable]
pub fn document_window(#[uncached] document: &mut Document) -> WidgetPod {
    //
    let menu_bar = main_menu_bar(document);

    // TODO document title
    Window::new(
        WindowBuilder::new().with_title("Document"),
        document_window_contents(document),
        Some(menu_bar),
    )
}

fn try_open_document() -> anyhow::Result<Document> {
    Ok(Document::open(Connection::open("test.artifice")?)?)
}

/// Application root.
#[composable(uncached)]
pub fn application_root() -> WidgetPod {
    let (document, key) = Cache::take_state::<Option<Document>>();
    let mut document = document.unwrap_or_default();

    let mut invalidate = false;
    let old_revision: Option<usize> = document.as_ref().map(|doc| doc.revision());

    let widget = if let Some(ref mut document) = document {
        let widget = document_window(document);
        invalidate = Some(document.revision()) != old_revision;
        widget
    } else {
        // create document
        // TODO open file dialog
        let window_contents: WidgetPod = match try_open_document() {
            Ok(new_document) => {
                document = Some(new_document);
                invalidate = true;
                Flex::new(Axis::Vertical, vec![])
            }
            Err(e) => {
                // error message
                Text::new(format!("Could not open file: {}", e))
            }
        };

        Window::new(
            WindowBuilder::new().with_title("No document"),
            window_contents,
            None,
        )
    };

    if invalidate {
        Cache::replace_state(key, document);
    } else {
        Cache::replace_state_without_invalidation(key, document);
    }

    widget
}
