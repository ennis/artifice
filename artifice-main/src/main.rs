use artifice::view;
use kyute::{application, shell::platform::Platform};

fn main() {
    let _platform = Platform::new();
    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        //.with_level(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        //.with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();

    application::run(view::application_root);
    Platform::shutdown();
}
