use crate::model::{Document, ModelPath, Node};
use kyute::{
    cache, composable,
    shell::{drawing::Color, winit::window::WindowBuilder},
    text::{Attribute, FontFamily, FontStyle, FormattedText, ParagraphStyle, TextStyle},
    widget::{
        Action, Axis, Baseline, Button, DropDown, Flex, Menu, MenuItem, Shortcut, Slider, Text,
        TextEdit,
    },
    Cache, Data, Key, WidgetPod, Window,
};
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
pub fn node_item(#[uncached] document: &mut Document, node: &Node) -> impl Widget {
    let delete_button = Button::new("Delete".to_string());
    if delete_button.clicked() {
        eprintln!("delete node clicked {:?}", node.base.path);
        document.delete_node(node);
    }

    //let name_edit = TextEdit::new(node.base.path.name().to_string());

    // problem: TextEdit is recreated every time a character is entered.
    // What we want:
    // - TextEdit::new() creates the text edit
    // - internally, text edit updates its internal document (selection, cursor movement).
    // - when enter is pressed, or focus is lost, invalidate the `EditingFinished` flag.
    //
    // Problem: we may also want to update:
    // - when the text changes.
    // - when the current selection changes.
    // - when the current cursor position changes.

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

    if let Some(item) = dropdown.new_selected_item() {
        eprintln!("changed option: {:?}", item);
    }

    Flex::new(
        Axis::Horizontal).append(
            Baseline::new(
                20.0,
                Text::new(format!("{}({})", node.base.path.to_string(), node.base.id)),
            ))
        .append(Baseline::new(20.0, delete_button))
        .append(Baseline::new(20.0, dropdown))
        .append( Baseline::new(20.0, name_edit))
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
            cache::scoped(node.base.id as usize, || {
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
        let slider = Slider::new(0.0, 10.0, 0.0);
        //eprintln!("slider value = {}", slider.current_value());
        flex_items.push(slider);
        flex_items
    };

    // enclosing window
    Flex::new(Axis::Vertical, flex_items)
}

/// Main menu bar.
#[composable]
pub fn main_menu_bar(#[uncached] document: &mut Document) -> Menu {
    // TODO macro to make shortcuts less verbose
    // `kyute::shortcut!(Ctrl+S)`
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

    eprintln!("document_window");
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
        eprintln!("invalidating document");
        document_state.set(document);
    } else {
        document_state.set_without_invalidation(document);
    }

    widget
}
