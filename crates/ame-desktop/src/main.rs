#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod action;
mod component;
mod entity;
mod gpui_http;
mod kernel;
mod tray;
mod view;

use crate::action::ui_actions::{
    HotkeyDiscover, HotkeyHome, HotkeyLibrary, HotkeyNextTrack, HotkeyPrevTrack, HotkeyQueue,
    HotkeyQuit, HotkeySearch, HotkeySettings, HotkeyTogglePlay,
};
use ame_core::init_logger;
use anyhow::Result;
use gpui::{
    App, AppContext, Application, AssetSource, Bounds, KeyBinding, SharedString, TitlebarOptions,
    WindowBounds, WindowOptions, px, size,
};
use std::{borrow::Cow, collections::HashSet, fs, path::PathBuf};
use tracing::error;

struct Assets {
    app_base: PathBuf,
    shared_base: PathBuf,
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        for base in [&self.app_base, &self.shared_base] {
            let file = base.join(path);
            if file.exists() {
                return fs::read(file)
                    .map(|data| Some(Cow::Owned(data)))
                    .map_err(Into::into);
            }
        }
        Ok(None)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut seen = HashSet::<String>::new();
        let mut results = Vec::new();

        for base in [&self.app_base, &self.shared_base] {
            if let Ok(entries) = fs::read_dir(base.join(path)) {
                for entry in entries.flatten() {
                    if let Ok(name) = entry.file_name().into_string()
                        && seen.insert(name.clone())
                    {
                        results.push(SharedString::from(name));
                    }
                }
            }
        }

        Ok(results)
    }
}

fn main() {
    init_logger();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Application::new()
        .with_assets(Assets {
            app_base: manifest_dir.join("assets"),
            shared_base: manifest_dir.join("../../resouses"),
        })
        .run(|cx: &mut App| {
            let http_client = match gpui_http::build_http_client("ame-app/0.0.6") {
                Ok(client) => Some(client),
                Err(err) => {
                    error!("set http client failed: {err}");
                    None
                }
            };
            if let Some(http_client) = http_client {
                cx.set_http_client(http_client);
            }

            gpui_router::init(cx);
            tray::init(cx);
            component::input::init_keybindings(cx);
            cx.bind_keys([
                KeyBinding::new("space", HotkeyTogglePlay, None),
                KeyBinding::new("ctrl-right", HotkeyNextTrack, None),
                KeyBinding::new("ctrl-left", HotkeyPrevTrack, None),
                KeyBinding::new("ctrl-1", HotkeyHome, None),
                KeyBinding::new("ctrl-2", HotkeyDiscover, None),
                KeyBinding::new("ctrl-3", HotkeyLibrary, None),
                KeyBinding::new("ctrl-f", HotkeySearch, None),
                KeyBinding::new("ctrl-j", HotkeyQueue, None),
                KeyBinding::new("ctrl-comma", HotkeySettings, None),
                KeyBinding::new("cmd-q", HotkeyQuit, None),
            ]);

            let bounds = Bounds::centered(None, size(px(1440.), px(840.)), cx);
            let options = WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Ame".into()),
                    appears_transparent: true,
                    traffic_light_position: None,
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(1080.), px(720.))),
                ..Default::default()
            };

            match cx.open_window(options, |window, cx| {
                let root = cx.new(|cx| view::root::RootView::new(window, cx));
                tray::set_main_root(cx, root.downgrade());
                tray::set_kernel_commands(cx, root.read(cx).kernel_command_sender());
                let weak = root.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    let _ = weak.update(cx, |root, cx| root.request_window_close(window, cx));
                    false
                });
                root
            }) {
                Ok(window) => tray::set_main_window(cx, window),
                Err(err) => error!("open window failed: {err}"),
            }

            cx.activate(true);
        });
}
