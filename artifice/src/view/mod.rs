use crate::model::{Document, ModelPath, Node};
use kyute::{cache, composable, shell::{drawing::Color, winit::window::WindowBuilder}, text::{Attribute, FontFamily, FontStyle, FormattedText, ParagraphStyle, TextStyle}, widget::{
    Action, Orientation, Baseline, Button, DropDown, Flex, Menu, MenuItem, Shortcut, Slider, Text,
    TextEdit,
}, Cache, Data, Key, Widget, WidgetPod, Window, State};
use rusqlite::Connection;
use std::{fmt, fmt::Formatter, sync::Arc};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data)]
enum DropDownTest {
    First,
    Second,
    Third,
}

impl fmt::Display for DropDownTest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Node view.
#[composable]
pub fn node_item(#[uncached] document: &mut Document, node: &Node) -> impl Widget + Clone {
    let delete_button = Button::new("Delete".to_string());
    if delete_button.clicked() {
        tracing::info!("delete node clicked {:?}", node.base.path);
        document.delete_node(node);
    }

    // format name
    let path = node.base.path.to_string();
    let last_sep = path.rfind('/').unwrap();
    let path_text = FormattedText::from(path)
        .with_attribute(
            0..=last_sep,
            Attribute::Color(Color::new(0.7, 0.7, 0.7, 1.0)),
        )
        .with_attribute(.., Attribute::FontSize(17.0))
        .with_attribute(.., FontFamily::new("Cambria"))
        .with_attribute(.., FontStyle::Italic);

    // rename
    let name_edit = TextEdit::new(path_text);

    let dropdown = DropDown::new(
        vec![
            DropDownTest::First,
            DropDownTest::Second,
            DropDownTest::Third,
        ],
        0,
    );

    if let Some(item) = dropdown.selected_item_changed() {
        tracing::info!("changed option: {:?}", item);
    }

    Flex::horizontal()
        .with(Baseline::new(
            30.0,
            Text::new(format!("{}({})", node.base.path.to_string(), node.base.id)),
        ))
        .with(Baseline::new(30.0, delete_button))
        .with(Baseline::new(30.0, dropdown))
        .with(Baseline::new(30.0, name_edit))
}

/// Root document view.
#[composable]
pub fn document_window_contents(#[uncached] document: &mut Document) -> WidgetPod {
    tracing::trace!("document_window_contents");

    let document_model = document.model().clone();

    let mut flex = Flex::vertical();

    // Root nodes

    for (_name, node) in document_model.root.children.iter() {
        cache::scoped(node.base.id as usize, || {
            flex.push(node_item(document, node));
        })
    }

    // "Add Node" button
    let add_node_button = Button::new("Add Node".to_string());

    if add_node_button.clicked() {
        tracing::info!("add node clicked");
        let name = document_model.root.make_unique_child_name("node");
        document.create_node(ModelPath::root().join(name));
    }

    flex.push(add_node_button);
    let slider_value = State::new(|| 0.0);
    let slider = Slider::new(0.0, 10.0, slider_value.get());
    slider_value.update(slider.value_changed());
    flex.push(slider);
    WidgetPod::new(flex)
}

/// Main menu bar.
#[composable]
pub fn main_menu_bar(#[uncached] document: &mut Document) -> Menu {
    let file_new = Action::with_shortcut(Shortcut::from_str("Ctrl+N"));
    let file_open = Action::with_shortcut(Shortcut::from_str("Ctrl+O"));
    let file_save = Action::with_shortcut(Shortcut::from_str("Ctrl+S"));
    let file_save_as = Action::with_shortcut(Shortcut::from_str("Ctrl+Shift+S"));
    let file_quit = Action::with_shortcut(Shortcut::from_str("Alt+F4"));
    let edit_undo = Action::with_shortcut(Shortcut::from_str("Ctrl+Z"));
    let edit_redo = Action::with_shortcut(Shortcut::from_str("Ctrl+Y"));

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

    tracing::trace!("document_window");
    let menu_bar = main_menu_bar(document);

    // TODO document title
    WidgetPod::new(Window::new(
        WindowBuilder::new().with_title("Document"),
        document_window_contents(document),
        Some(menu_bar),
    ))
}

fn try_open_document() -> anyhow::Result<Document> {
    Ok(Document::open(Connection::open("test.artifice")?)?)
}

/// Application root.
#[composable(uncached)]
pub fn application_root() -> WidgetPod {
    let document_state = cache::state(|| -> Option<Document> { None });
    let mut document = document_state.take();

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
                WidgetPod::new(Flex::vertical())
            }
            Err(e) => {
                // error message
                WidgetPod::new(Text::new(format!("Could not open file: {}", e)))
            }
        };

        WidgetPod::new(Window::new(
            WindowBuilder::new().with_title("No document"),
            window_contents,
            None,
        ))
    };

    if invalidate {
        tracing::trace!("invalidating document");
        document_state.set(document);
    } else {
        document_state.set_without_invalidation(document);
    }

    widget
}
