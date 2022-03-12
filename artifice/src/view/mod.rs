use crate::model::{Document, ModelPath, Node};
use kyute::{
    cache, composable,
    shell::winit::window::WindowBuilder,
    style::BoxStyle,
    text::{Attribute, FontFamily, FontStyle, FormattedText},
    theme,
    widget::{
        drop_down, grid,
        grid::{GridRow, GridTrackDefinition},
        Action, Baseline, Button, Container, DropDown, Flex, Grid, GridLength, Image, Label, Menu, MenuItem, Null,
        Orientation, Shortcut, Slider, TextEdit,
    },
    Color, Data, State, Widget, WidgetPod, Window,
};
use kyute_common::Length;
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

const LABEL_COLUMN: &str = "label";
const ADD_COLUMN: &str = "add";
const DELETE_COLUMN: &str = "delete";
const VALUE_COLUMN: &str = "value";

/// Node view.
#[composable]
pub fn node_item(document: &mut Document, node: &Node) -> GridRow<'static> {
    let delete_button = Button::new("Delete".to_string());

    if delete_button.clicked() {
        tracing::info!("delete node clicked {:?}", node.base.path);
        document.delete_node(node);
    }

    /*let result = cache::enqueue(async move || {
        wait(300.ms()).await;
        // do stuff
        wait(300.ms()).await;
    });

    if asset_changed(asset_id) {
        // reload asset?
    }

    if let Some(result) = result {
        // future (or stream) finished (or produced something)
    }*/

    // animations:
    // will recomp at some fixed intervals for 300ms, producing a different value every time
    // (pray that we don't allocate too much)
    //let y_pos = animate(delete_button.clicked(), 300.ms(), 0.0, 100.0, Easing::InOut);

    // state machines:
    // e.g.
    // click -> state1, click again -> state2

    // format name
    let path = node.base.path.to_string();
    let last_sep = path.rfind('/').unwrap();
    let path_text = FormattedText::from(path)
        .attribute(0..=last_sep, Attribute::Color(Color::new(0.7, 0.7, 0.7, 1.0)))
        .attribute(.., Attribute::FontSize(17.0))
        .attribute(.., FontFamily::new("Cambria"))
        .attribute(.., FontStyle::Italic);

    // rename
    let name_edit = TextEdit::new(path_text);

    let dropdown = DropDown::with_selected(
        DropDownTest::First,
        vec![DropDownTest::First, DropDownTest::Second, DropDownTest::Third],
        drop_down::DebugFormatter,
    );

    if let Some(item) = dropdown.selected_item_changed() {
        tracing::info!("changed option: {:?}", item);
    }

    let mut row = GridRow::new();
    row.add(
        LABEL_COLUMN,
        Label::new(format!("{}({})", node.base.path.to_string(), node.base.id)),
    );
    row.add(DELETE_COLUMN, delete_button);
    row.add(ADD_COLUMN, dropdown);
    row.add(VALUE_COLUMN, name_edit);
    row
}

/// Root document view.
#[composable(cached)]
pub fn document_window_contents(#[uncached] document: &mut Document) -> impl Widget + Clone {
    #[state]
    let mut slider_value = 0.0;

    tracing::trace!("document_window_contents");
    let document_model = document.model().clone();

    let mut grid = Grid::with_column_definitions([
        GridTrackDefinition::named(LABEL_COLUMN, GridLength::Fixed(100.0)),
        GridTrackDefinition::named(DELETE_COLUMN, GridLength::Fixed(60.0)),
        GridTrackDefinition::named(ADD_COLUMN, GridLength::Fixed(60.0)),
        GridTrackDefinition::named(VALUE_COLUMN, GridLength::Flex(1.0)),
    ])
    .align_items(grid::AlignItems::Baseline)
    .row_gap(Length::Px(2.0));

    // Root nodes
    for (_name, node) in document_model.root.children.iter() {
        cache::scoped(node.base.id as usize, || {
            grid.add_row(node_item(document, node));
        })
    }

    // "Add Node" button
    let add_node_button = Button::new("Add Node".to_string());
    if add_node_button.clicked() {
        tracing::info!("add node clicked");
        let name = document_model.root.make_unique_child_name("node");
        document.create_node(ModelPath::root().join(name));
    }
    grid.add_item(grid.row_count(), 0, add_node_button);

    // Slider test
    let slider = Slider::new(0.0, 10.0, slider_value).on_value_changed(|v| slider_value = v);
    grid.add_item(grid.row_count(), .., slider);

    let container = Container::new(grid).box_style(BoxStyle::new().fill(theme::keys::UNDER_PAGE_BACKGROUND_COLOR));

    Arc::new(container)
}

/// Main menu bar.
#[composable(cached)]
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
    let mut edit_menu = Menu::new(vec![MenuItem::new("Undo", edit_undo), MenuItem::new("Redo", edit_redo)]);
    let menu_bar = Menu::new(vec![
        MenuItem::submenu("File", file_menu),
        MenuItem::submenu("Edit", edit_menu),
    ]);

    menu_bar
}

/// Native window displaying a document.
#[composable(cached)]
pub fn document_window(#[uncached] document: &mut Document) -> Window {
    //
    tracing::trace!("document_window");
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
#[composable]
pub fn application_root() -> Arc<WidgetPod> {
    let document_state = cache::state(|| -> Option<Document> { None });
    let mut document = document_state.take_without_invalidation();

    let mut invalidate = false;
    let old_revision: Option<usize> = document.as_ref().map(|doc| doc.revision());
    let window = if let Some(ref mut document) = document {
        let window = WidgetPod::new(document_window(document));
        invalidate = Some(document.revision()) != old_revision;
        window
    } else {
        // create document
        // TODO open file dialog
        let window_contents: Arc<dyn Widget> = match try_open_document() {
            Ok(new_document) => {
                document = Some(new_document);
                invalidate = true;
                Arc::new(Flex::new(Orientation::Vertical))
            }
            Err(e) => {
                // error message
                Arc::new(Label::new(format!("Could not open file: {}", e)))
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

    Arc::new(window)
}
