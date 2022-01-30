use artifice::view;
use kyute::{application, shell::application::Application, theme, SHOW_DEBUG_OVERLAY};

fn main() {
    let _app = Application::new();

    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        //.with_level(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        //.with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();

    application::run(
        view::application_root,
        theme::get_default_application_style().add(SHOW_DEBUG_OVERLAY, true),
    );

    Application::shutdown();
}
