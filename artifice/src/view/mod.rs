//mod toolbar;

use crate::model::{Document, DocumentEditProxy, DocumentFile, Node, NodeEditProxy, Param, Path};
use kyute::{
    cache, composable,
    shell::{
        text::{Attribute, FontFamily, FontStyle, FormattedText},
        winit::window::WindowBuilder,
    },
    theme,
    widget::{
        drop_down, grid,
        grid::{GridTemplate, SHOW_GRID_LAYOUT_LINES},
        Action, Button, ColumnHeaders, DropDown, Flex, Grid, Image, Label, Menu, MenuItem, Null, Orientation, Shortcut,
        TableRow, TableView, TableViewParams, Text, WidgetPod,
    },
    Color, Data, Font, Length, State, UnitExt, Widget, Window,
};
use rusqlite::Connection;
use std::{convert::TryFrom, fmt, fmt::Formatter, sync::Arc};

const LABEL_COLUMN: &str = "label";
const ADD_COLUMN: &str = "add";
const DELETE_COLUMN: &str = "delete";
const VALUE_COLUMN: &str = "value";

/// Attribute row
#[composable(cached)]
pub fn attribute_row(attribute: &Param, #[uncached] edit: &mut DocumentEditProxy) -> TableRow<i64> {
    // format attribute name
    let attr_name = attribute.path.name();
    let mut label = FormattedText::new(attr_name.as_ref().into());
    if let Some(p) = attr_name.rfind(':') {
        label.add_attribute(0..p + 1, Attribute::Color(theme::palette::GREY_50.with_alpha(0.5)));
    }

    let mut row = TableRow::new(attribute.id, Text::new(label));
    row.add_cell(1, Label::new(attribute.ty.to_string()));
    row.add_cell(2, Label::new(format!("{:?}", attribute.value)));
    row
}

/// Node view.
#[composable(cached)]
pub fn node_row(node: &Node, #[uncached] edit: &mut DocumentEditProxy) -> TableRow<i64> {
    let label = if node.path.is_root() {
        FormattedText::from("<root node>").attribute(.., FontStyle::Italic)
    } else {
        FormattedText::from(node.name().to_string())
    };

    let mut row = TableRow::new(node.id, Text::new(label));

    for attr in node.attributes.values() {
        cache::scoped(attr.id, || {
            row.add_row(attribute_row(attr, edit));
        });
    }

    for node in node.children.values() {
        cache::scoped(node.id, || {
            row.add_row(node_row(node, edit));
        });
    }

    row

    /*let delete_button = Button::new("Delete".to_string()).on_clicked(|| {
        tracing::info!("delete node clicked {:?}", node.path);
        edit.remove(&node.path);
    });*/

    /*// format name
    let path = node.path.to_string();
    let last_sep = path.rfind('/').unwrap();
    let path_text = FormattedText::from(path)
        .attribute(0..=last_sep, Attribute::Color(Color::new(0.7, 0.7, 0.7, 1.0)))
        .attribute(.., Attribute::FontSize(17.0))
        .attribute(.., FontFamily::new("Cambria"))
        .attribute(.., FontStyle::Italic);

    let mut row = GridRow::new();
    row.add(
        LABEL_COLUMN,
        Label::new(format!("{}({})", node.path.to_string(), node.id)),
    );
    row.add(DELETE_COLUMN, delete_button);
    row.add(ADD_COLUMN, dropdown);
    row.add(VALUE_COLUMN, name_edit);
    row*/
}

/// Root document view.
#[composable(cached, live_literals)]
pub fn document_window_contents(document: &Document, #[uncached] edit: &mut DocumentEditProxy) -> impl Widget + Clone {
    tracing::trace!("document_window_contents");

    let root_row = node_row(&document.root, edit);

    let table_params = TableViewParams {
        selection: None,
        template: GridTemplate::try_from("{auto} / 200 200 1fr").unwrap(),
        column_headers: Some(
            ColumnHeaders::new()
                .add(Text::new("Name"))
                .add(Text::new("Type"))
                .add(Text::new("Value")),
        ),
        main_column: 0,
        rows: vec![root_row],
        row_indent: Length::Dip(20.0),
        resizeable_columns: false,
        reorderable_rows: false,
        reorderable_columns: false,
        background: theme::palette::GREY_800.into(),
        alternate_background: theme::palette::GREY_700.into(),
        row_separator_width: Default::default(),
        column_separator_width: Default::default(),
        row_separator_background: Default::default(),
        column_separator_background: Default::default(),
        selected_style: Default::default(),
    };

    let table = TableView::new(table_params);

    let mut grid = Grid::with_template("auto auto / 1fr");
    //let toolbar = Toolbar::new()
    //    .text_icon_button("Create", "data/icons/file_new.png")
    //   .text_icon_button("Open", "data/icons/file_folder.png")
    //    .text_icon_button("Save", "data/icons/file_tick.png");
    //grid.insert(toolbar);
    grid.insert(table);
    Arc::new(grid)
}

/// Main menu bar.
#[composable(cached)]
pub fn main_menu_bar(document: &Document) -> Menu {
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
pub fn document_window(document: &Document, #[uncached] edit: &mut DocumentEditProxy) -> Window {
    //
    tracing::trace!("document_window");
    let menu_bar = main_menu_bar(document);

    // TODO document title
    Window::new(
        WindowBuilder::new().with_title("Document"),
        document_window_contents(document, edit),
        Some(menu_bar),
    )
}

fn try_open_document() -> anyhow::Result<DocumentFile> {
    Ok(DocumentFile::open(Connection::open("scene_info.db")?)?)
}

/// Application root.
#[composable]
pub fn application_root() -> impl Widget {
    let document_file_state = cache::state(|| Some(try_open_document().unwrap()));
    let mut document_file = document_file_state.take_without_invalidation().unwrap();

    let rev = document_file.revision();
    let window = document_file.edit(|document, edit| document_window(document, edit));
    if document_file.revision() != rev {
        document_file_state.set(Some(document_file));
    } else {
        document_file_state.set_without_invalidation(Some(document_file));
    }

    window
}
