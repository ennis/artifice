use kyute::{
    composable,
    style::{BoxStyle, LinearGradient, Paint},
    theme, tweak,
    widget::{grid::FlowDirection, Container, Grid, GridLength, Image, Scaling, Text, WidgetWrapper},
    WidgetExt,
};
use kyute_common::{Color, UnitExt};

struct ToolbarItem {}

#[derive(WidgetWrapper)]
pub struct Toolbar {
    inner: Container<Grid>,
}

impl Toolbar {
    #[composable(live_literals)]
    pub fn new() -> Toolbar {
        let mut grid = Grid::with_template("40 20 / {55} [end] / 5 10");
        grid.set_auto_flow(FlowDirection::Column);

        let inner = Container::new(grid)
            .background("linear-gradient(90deg, #D7D5D7, #F6F5F6)")
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
        grid.insert((
            // Icon
            Image::from_uri(icon_uri, Scaling::Contain).fix_size(32.dip(), 32.dip()),
            // Text placeholder
            (),
        ));
        self
    }

    #[composable]
    pub fn text_icon_button(mut self, label: impl Into<String>, icon_uri: &str) -> Self {
        let grid = self.inner.inner_mut();
        grid.insert((
            Image::from_uri(icon_uri, Scaling::Contain)
                .colorize(theme::palette::GREY_800)
                .fix_size(32.dip(), 32.dip())
                .centered(),
            Text::new(label.into()).color(theme::palette::GREY_800).centered(),
        ));
        self
    }
}
