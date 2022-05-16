use kyute::{
    composable,
    style::{BoxStyle, LinearGradient, Paint},
    theme,
    widget::{grid::GridTrackDefinition, Container, Grid, GridLength, Image, Scaling, Text, WidgetWrapper},
    WidgetExt,
};
use kyute_common::{Color, UnitExt};

struct ToolbarItem {}

#[derive(WidgetWrapper)]
pub struct Toolbar {
    inner: Container<Grid>,
}

impl Toolbar {
    #[composable]
    pub fn new() -> Toolbar {
        let mut grid = Grid::new();
        grid.push_row_definition(GridTrackDefinition::new(GridLength::Fixed(45.dip())));
        grid.set_row_gap(5.dip());
        grid.push_row_definition(GridTrackDefinition::new(GridLength::Fixed(20.dip())));
        grid.set_column_gap(10.dip());
        grid.set_column_template(GridLength::Fixed(80.dip()));
        let inner = Container::new(grid)
            .background(Paint::from(
                LinearGradient::new()
                    .angle(90.degrees())
                    .stop(Color::from_hex("#D7D5D7"), 0.0)
                    .stop(Color::from_hex("#F6F5F6"), 1.0),
            ))
            .content_padding(10.dip(), 10.dip(), 10.dip(), 10.dip())
            .centered();
        Toolbar { inner }
    }

    #[composable]
    pub fn text_button(mut self, label: impl Into<String>) -> Self {
        self
    }

    #[composable]
    pub fn icon_button(mut self, icon_uri: &str) -> Self {
        let grid = self.inner.inner_mut();
        let cols = grid.column_count();
        grid.add_item(
            0,
            cols,
            0,
            Image::from_uri(icon_uri, Scaling::Contain).fix_size(32.dip(), 32.dip()),
        );
        self
    }

    #[composable]
    pub fn text_icon_button(mut self, label: impl Into<String>, icon_uri: &str) -> Self {
        let grid = self.inner.inner_mut();
        let cols = grid.column_count();
        grid.add_item(
            0,
            cols,
            0,
            Image::from_uri(icon_uri, Scaling::Contain)
                .colorize(theme::palette::GREY_800)
                .fix_size(32.dip(), 32.dip())
                .centered(),
        );
        grid.add_item(
            1,
            cols,
            0,
            Text::new(label.into()).color(theme::palette::GREY_800).centered(),
        );
        self
    }
}
