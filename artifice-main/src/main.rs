use artifice::view;
use kyute::{application, shell::application::Application, theme, Environment, SHOW_DEBUG_OVERLAY};

fn main() {
    let _app = Application::new();

    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        //.with_level(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        //.with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();

    let mut env = Environment::new();
    theme::setup_default_style(&mut env);
    env.set(SHOW_DEBUG_OVERLAY, true);
    env.set(kyute::widget::grid::SHOW_GRID_LAYOUT_LINES, true);

    application::run(view::application_root, env);
    Application::shutdown();
}
