use kyute::{
    composable,
    style::{BoxStyle, LinearGradient, Paint},
    theme, tweak,
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
        let mut grid = Grid::with_spec(tweak!("40 20 / {55} [end] / 5 10"));

        /*grid.push_row_definition(GridTrackDefinition::new(GridLength::Fixed(tweak!(40).dip())));
        grid.set_row_gap(tweak!(5).dip());
        grid.push_row_definition(GridTrackDefinition::new(GridLength::Fixed(tweak!(20).dip())));
        grid.set_column_gap(tweak!(10).dip());
        grid.set_column_template(GridLength::Fixed(tweak!(55).dip()));*/

        let inner = Container::new(grid)
            .background(Paint::from(
                LinearGradient::new()
                    .angle(tweak!(90).degrees())
                    .stop(Color::from_hex(tweak!("#D7D5D7")), 0.0)
                    .stop(Color::from_hex(tweak!("#F6F5F6")), 1.0),
            ))
            .content_padding(tweak!(10).dip(), tweak!(10).dip(), tweak!(10).dip(), tweak!(10).dip())
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
        grid.append_column();
        //let cols = grid.column_count();
        grid.add_item(
            "0 / -1",
            Image::from_uri(icon_uri, Scaling::Contain)
                .colorize(theme::palette::GREY_800)
                .fix_size(32.dip(), 32.dip())
                .centered(),
        );
        grid.add_item(
            "0 / -1",
            Text::new(label.into()).color(theme::palette::GREY_800).centered(),
        );
        self
    }
}
