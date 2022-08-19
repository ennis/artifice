use artifice::view;
use kyute::{application, shell::application::Application, theme, Environment, SHOW_DEBUG_OVERLAY};

fn main() {
    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let mut env = Environment::new();
    theme::setup_default_style(&mut env);
    //env.set(SHOW_DEBUG_OVERLAY, true);
    application::run_with_env(view::application_root, env);
}
