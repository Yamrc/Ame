mod action;
mod component;
mod entity;
mod view;

use ame_core::init_logger;
use gpui::Application;
use gpui_tray::{Tray, TrayAppContext};
use tracing::info;

fn main() {
    init_logger();

    Application::new().run(|cx| {
        let _app_state = entity::app::AppEntity::default();
        info!("ame app booted with MVP module graph");
        cx.set_tray(Tray::new()).unwrap();
    });
}
