use kyute::{
    application, composable,
    shell::application::Application,
    style::{ThemeData, UnitExt},
    theme,
    widget::{Button, ConstrainedBox, Container, Flex, Grid, GridLength, Image, Label, Null},
    Alignment, BoxConstraints, Color, EnvKey, Environment, Orientation, Size, Widget, WidgetExt,
    WidgetPod, Window,
};
use kyute_shell::{winit::window::WindowBuilder, AssetId};
use std::sync::Arc;

#[composable]
fn fixed_size_widget(w: f64, h: f64, name: &str) -> impl Widget {
    // TODO "debug widget" that draws a background pattern, with a border
    (#[compose]
    Label::new(name.to_string()))
    .fix_size(Size::new(w, h))
}

#[composable]
fn grid_layout_example() -> impl Widget + Clone {
    let mut grid = Grid::with_rows_columns(
        [GridLength::Fixed(100.0), GridLength::Flex(1.0)],
        [
            GridLength::Fixed(100.0),
            GridLength::Auto,
            GridLength::Fixed(100.0),
        ],
    );

    #[compose]
    grid.add(
        0,
        0,
        #[compose]
        fixed_size_widget(50.0, 50.0, "(0,0)"),
    );

    #[compose]
    grid.add(
        0,
        1,
        #[compose]
        fixed_size_widget(50.0, 50.0, "(0,1)"),
    );
    //grid.add(0, 2, fixed_size_widget(50.0, 50.0, "(0,2)"));

    #[compose]
    grid.add(
        0,
        2,
        #[compose]
        Image::from_uri_async("data/haniyasushin_keiki.jpg", Null),
    );

    #[compose]
    grid.add(
        1,
        0,
        #[compose]
        fixed_size_widget(50.0, 50.0, "(1,0)"),
    );

    #[compose]
    grid.add(
        1,
        1..=2,
        (#[compose]
        fixed_size_widget(150.0, 50.0, "(1,1)"))
        .centered(),
    );

    grid
}

#[composable]
fn align_in_constrained_box() -> impl Widget + Clone {
    use kyute::style::*;

    let mut grid = Grid::column(GridLength::Auto);

    #[compose]
    grid.add_row(
        (#[compose]
        Label::new("ConstrainedBox".into()))
        .aligned(Alignment::CENTER_RIGHT)
        .height_factor(1.0)
        .fix_width(300.0),
    );

    #[compose]
    grid.add_row(
        #[compose]
        grid_layout_example(),
    );

    #[compose]
    grid.add_row(
        Container::new(
            #[compose]
            Label::new("Container".into()),
        )
        //.aligned(Alignment::CENTER_RIGHT)
        .fixed_width(500.dip())
        .box_style(BoxStyle::new().fill(Color::from_hex("#b9edc788"))),
    );

    grid
}

#[composable]
fn ui_root() -> Arc<WidgetPod> {
    Arc::new(
        #[compose]
        WidgetPod::new(
            #[compose]
            Window::new(
                WindowBuilder::new().with_title("Layouts"),
                #[compose]
                Flex::vertical().with(
                    #[compose]
                    align_in_constrained_box(),
                ),
                None,
            ),
        ),
    )
}

fn main() {
    let _app = Application::new();

    let mut env = Environment::new();
    theme::setup_default_style(&mut env);
    env.set(kyute::widget::grid::SHOW_GRID_LAYOUT_LINES, true);

    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    application::run(ui_root, env);

    Application::shutdown();
}
