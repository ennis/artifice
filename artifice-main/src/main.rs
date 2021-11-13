
use artifice::data::AppData;
use artifice::view::app;

fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .init();

    let main_window = WindowDesc::new(app::ui())
        .menu(|_,_,_| app::application_menu())
        .title("Artifice");

    let data = AppData::new();
    AppLauncher::with_window(main_window)
        .delegate(app::Delegate)
        .launch(data)
        .expect("launch failed");
}
