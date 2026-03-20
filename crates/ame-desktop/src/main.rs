#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod animation;
mod app;
mod component;
mod domain;
mod gpui_http;
mod page;
mod util;

use crate::app::hotkeys::{
    HotkeyDiscover, HotkeyHome, HotkeyLibrary, HotkeyNextTrack, HotkeyPrevTrack, HotkeyQueue,
    HotkeyQuit, HotkeySearch, HotkeySettings, HotkeyTogglePlay,
};
use ame_core::init_logger;
use anyhow::Result;
use nekowg::{
    App, AppContext, AssetSource, Bounds, KeyBinding, SharedString, TitlebarOptions, WindowBounds,
    WindowOptions, px, size,
};
use nekowg_platform::application;
use rust_embed::RustEmbed;
use std::{borrow::Cow, collections::HashSet};
use tracing::warn;

#[derive(RustEmbed)]
#[folder = "../../resouses"]
struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(Self::get(path).map(|data| data.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut seen = HashSet::<String>::new();
        let mut results = Vec::new();
        let normalized = path.trim_matches('/');
        let prefix = if normalized.is_empty() {
            String::new()
        } else {
            format!("{normalized}/")
        };

        for asset_path in Self::iter() {
            if let Some(rest) = asset_path.strip_prefix(&prefix)
                && !rest.is_empty()
            {
                let name = rest.split('/').next().unwrap_or_default();
                if !name.is_empty() && seen.insert(name.to_string()) {
                    results.push(SharedString::from(name.to_string()));
                }
            }
        }

        Ok(results)
    }
}

fn main() {
    init_logger();
    application()
        .with_assets(Assets)
        .with_http_client(gpui_http::build_http_client("ame/1").expect("set http client failed"))
        .with_quit_mode(nekowg::QuitMode::Explicit)
        .run(|cx: &mut App| {
            app::router::init(cx);
            app::tray::init(cx);
            component::input::init_keybindings(cx);
            cx.bind_keys([
                KeyBinding::new("space", HotkeyTogglePlay, Some("!AmeInput")),
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

            let window = cx
                .open_window(options, |window, cx| {
                    let root = cx.new(|cx| app::root::RootView::new(window, cx));
                    app::tray::set_main_root(cx, root.downgrade());
                    let weak = root.downgrade();
                    window.on_window_should_close(cx, move |window, cx| {
                        if let Err(err) = weak.update(cx, |root: &mut app::root::RootView, cx| {
                            root.request_window_close(window, cx);
                        }) {
                            warn!("window close callback update failed: {err}");
                        }
                        false
                    });
                    root
                })
                .expect("Failed to open default windows");
            app::tray::set_main_window(cx, window);

            cx.activate(true);
        });
}
