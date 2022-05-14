use crate::model::{Document, Node, NodeEditProxy, Path};
use artifice::model::DocumentFile;
use kyute::{
    cache, composable,
    shell::{
        text::{Attribute, FontFamily, FontStyle, FormattedText},
        winit::window::WindowBuilder,
    },
    style::BoxStyle,
    theme,
    widget::{
        drop_down, grid,
        grid::{GridRow, GridTrackDefinition},
        Action, Baseline, Button, Container, DropDown, Flex, Grid, GridLength, Image, Label, Menu, MenuItem, Null,
        Orientation, Shortcut, Slider, TextEdit, WidgetPod,
    },
    Color, Data, State, Widget, Window,
};
use kyute_common::{Length, UnitExt};
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
pub fn node_item(node: &mut NodeEditProxy) -> GridRow<'static> {
    let delete_button = Button::new("Delete".to_string()).on_clicked(|| {
        tracing::info!("delete node clicked {:?}", node.path);
        node.remove();
    });

    // format name
    let path = node.path.to_string();
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
        Label::new(format!("{}({})", node.path.to_string(), node.id)),
    );
    row.add(DELETE_COLUMN, delete_button);
    row.add(ADD_COLUMN, dropdown);
    row.add(VALUE_COLUMN, name_edit);
    row
}

/// Root document view.
#[composable(cached)]
pub fn document_window_contents(document: &mut NodeEditProxy) -> impl Widget + Clone {
    #[state]
    let mut slider_value = 0.0;

    tracing::trace!("document_window_contents");

    let mut grid = Grid::new();
    grid.push_column_definition(GridTrackDefinition::named(LABEL_COLUMN, GridLength::Fixed(100.dip())));
    grid.push_column_definition(GridTrackDefinition::named(DELETE_COLUMN, GridLength::Fixed(60.dip())));
    grid.push_column_definition(GridTrackDefinition::named(ADD_COLUMN, GridLength::Fixed(60.dip())));
    grid.push_column_definition(GridTrackDefinition::named(VALUE_COLUMN, GridLength::Flex(1.0)));

    grid.set_align_items(grid::AlignItems::Baseline);
    grid.set_row_gap(2.px());

    // Root nodes
    document.edit_children(|node| {
        cache::scoped(node.id as usize, || {
            grid.add_row(node_item(node));
        })
    });

    // "Add Node" button
    let add_node_button = Button::new("Add Node".to_string());
    if add_node_button.clicked() {
        tracing::info!("add node clicked");
        let name = document.make_unique_child_name("node");
        document.get_or_create_node(name, |node| {});
    }
    grid.add_item(grid.row_count(), 0, 0, add_node_button);

    // Slider test
    let slider = Slider::new(0.0, 10.0, slider_value).on_value_changed(|v| slider_value = v);
    grid.add_item(grid.row_count(), .., 0, slider);

    let container = Container::new(grid).box_style(BoxStyle::new().fill(theme::palette::GREY_600));

    Arc::new(container)
}

/// Main menu bar.
#[composable(cached)]
pub fn main_menu_bar(document: &mut NodeEditProxy) -> Menu {
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
pub fn document_window(document: &mut NodeEditProxy) -> Window {
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

fn try_open_document() -> anyhow::Result<DocumentFile> {
    Ok(DocumentFile::open(Connection::open("test.artifice")?)?)
}

/// Application root.
#[composable]
pub fn application_root() -> impl Widget {
    let document_file_state = cache::state(|| Some(try_open_document().unwrap()));
    let mut document_file = document_file_state.take_without_invalidation().unwrap();

    let mut changed = false;
    let window = document_file.edit(|mut document| {
        let w = document_window(&mut document);
        if document.changed() {
            changed = true;
        }
        w
    });

    if changed {
        document_file_state.set(Some(document_file));
    } else {
        document_file_state.set_without_invalidation(Some(document_file));
    }

    window
}
