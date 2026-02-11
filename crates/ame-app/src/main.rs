use ame_core::init_logger;
use gpui::Application;

fn main() {
    init_logger();

    Application::new().run(|_| tracing::info!("AME running."));
}
