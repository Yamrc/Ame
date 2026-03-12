#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

mod action;
mod animation;
mod component;
mod entity;
mod gpui_http;
mod kernel;
mod router;
mod tray;
mod util;
mod view;

use crate::action::ui_actions::{
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
use tracing::error;

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
    application().with_assets(Assets).run(|cx: &mut App| {
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

        router::init(cx);
        tray::init(cx);
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

        match cx.open_window(options, |window, cx| {
            let root = cx.new(|cx| view::root::RootView::new(window, cx));
            tray::set_main_root(cx, root.downgrade());
            tray::set_kernel_commands(cx, root.read(cx).kernel_command_sender());
            let weak = root.downgrade();
            window.on_window_should_close(cx, move |window, cx| {
                let _ = weak.update(cx, |root: &mut view::root::RootView, cx| {
                    root.request_window_close(window, cx);
                });
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
