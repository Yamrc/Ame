use nekowg::actions;
use nekowg::{App, Global, Image, ImageFormat, MenuItem, MouseButton, WeakEntity, WindowHandle};
use nekowg_tray::{ClickEvent, DoubleClickEvent, Tray, TrayAppContext};
use tracing::{error, warn};

use crate::app::hotkeys::{
    HotkeyDiscover, HotkeyHome, HotkeyLibrary, HotkeyNextTrack, HotkeyPrevTrack, HotkeyQueue,
    HotkeyQuit, HotkeySearch, HotkeySettings, HotkeyTogglePlay,
};
use crate::app::root::RootView;
use crate::app::route::AppRoute;

actions!(
    tray_actions,
    [TrayShowWindow, TrayTogglePlay, TrayNext, TrayQuit]
);

#[derive(Default)]
pub struct AppWindows {
    pub main_window: Option<WindowHandle<RootView>>,
    pub main_root: Option<WeakEntity<RootView>>,
}

impl Global for AppWindows {}

pub fn init(cx: &mut App) {
    cx.set_global(AppWindows::default());
    cx.on_action(on_show_window);
    cx.on_action(on_toggle_play);
    cx.on_action(on_next);
    cx.on_action(on_quit);
    cx.on_action(on_tray_click);
    cx.on_action(on_tray_double_click);
    cx.on_action(on_hotkey_toggle_play);
    cx.on_action(on_hotkey_next_track);
    cx.on_action(on_hotkey_prev_track);
    cx.on_action(on_hotkey_home);
    cx.on_action(on_hotkey_discover);
    cx.on_action(on_hotkey_library);
    cx.on_action(on_hotkey_search);
    cx.on_action(on_hotkey_queue);
    cx.on_action(on_hotkey_settings);
    cx.on_action(on_hotkey_quit);

    let icon = Image::from_bytes(
        ImageFormat::Png,
        include_bytes!("../../../../resouses/image/icon.jpg").to_vec(),
    );
    let tray = Tray::new().tooltip("Ame").icon(icon).menu(|| {
        vec![
            MenuItem::action("显示主窗口", TrayShowWindow),
            MenuItem::separator(),
            MenuItem::action("播放/暂停", TrayTogglePlay),
            MenuItem::action("下一首", TrayNext),
            MenuItem::separator(),
            MenuItem::action("退出应用", TrayQuit),
        ]
    });

    if let Err(err) = cx.set_tray(tray) {
        error!("set tray failed: {err}");
    }
}

pub fn set_main_window(cx: &mut App, window: WindowHandle<RootView>) {
    cx.global_mut::<AppWindows>().main_window = Some(window);
}

pub fn set_main_root(cx: &mut App, root: WeakEntity<RootView>) {
    cx.global_mut::<AppWindows>().main_root = Some(root);
}

fn with_main_window(cx: &mut App, f: impl FnOnce(WindowHandle<RootView>, &mut App)) {
    let window = cx.global::<AppWindows>().main_window;
    if let Some(window) = window {
        f(window, cx);
    }
}

fn with_main_root(cx: &mut App, f: impl FnOnce(WeakEntity<RootView>, &mut App)) {
    let root = cx.global::<AppWindows>().main_root.clone();
    if let Some(root) = root {
        f(root, cx);
    }
}

fn on_show_window(_: &TrayShowWindow, cx: &mut App) {
    cx.activate(true);
    with_main_window(cx, |window, cx| {
        if let Err(err) = window.update(cx, |_, window, _| {
            window.show();
            window.activate_window();
        }) {
            error!("show window failed: {err}");
        }
    });
}

fn on_toggle_play(_: &TrayTogglePlay, cx: &mut App) {
    with_main_root(cx, |root, cx| {
        if let Err(err) = root.update(cx, |root, cx| root.tray_toggle_playback(cx)) {
            error!("tray toggle play failed: {err}");
        }
    });
}

fn on_next(_: &TrayNext, cx: &mut App) {
    with_main_root(cx, |root, cx| {
        if let Err(err) = root.update(cx, |root, cx| root.tray_next(cx)) {
            error!("tray next failed: {err}");
        }
    });
}

fn on_quit(_: &TrayQuit, cx: &mut App) {
    with_main_root(cx, |root, cx| {
        if let Err(err) = root.update(cx, |root, cx| root.prepare_app_exit(cx)) {
            error!("prepare exit failed: {err}");
        }
    });
    cx.quit();
}

fn on_tray_click(event: &ClickEvent, cx: &mut App) {
    if event.button == MouseButton::Left {
        on_show_window(&TrayShowWindow, cx);
    }
}

fn on_tray_double_click(_: &DoubleClickEvent, cx: &mut App) {
    on_show_window(&TrayShowWindow, cx);
}

fn navigate_with_root(cx: &mut App, route: AppRoute) {
    with_main_root(cx, |root, cx| {
        if let Err(err) = root.update(cx, |root, cx| root.navigate_to(route.clone(), cx)) {
            warn!("hotkey navigate failed: {err}");
        }
    });
}

fn on_hotkey_toggle_play(_: &HotkeyTogglePlay, cx: &mut App) {
    on_toggle_play(&TrayTogglePlay, cx);
}

fn on_hotkey_next_track(_: &HotkeyNextTrack, cx: &mut App) {
    on_next(&TrayNext, cx);
}

fn on_hotkey_prev_track(_: &HotkeyPrevTrack, cx: &mut App) {
    with_main_root(cx, |root, cx| {
        if let Err(err) = root.update(cx, |root, cx| root.tray_previous(cx)) {
            warn!("hotkey previous failed: {err}");
        }
    });
}

fn on_hotkey_home(_: &HotkeyHome, cx: &mut App) {
    navigate_with_root(cx, AppRoute::Home);
}

fn on_hotkey_discover(_: &HotkeyDiscover, cx: &mut App) {
    navigate_with_root(cx, AppRoute::Explore);
}

fn on_hotkey_library(_: &HotkeyLibrary, cx: &mut App) {
    navigate_with_root(cx, AppRoute::Library);
}

fn on_hotkey_search(_: &HotkeySearch, cx: &mut App) {
    navigate_with_root(cx, AppRoute::Search);
}

fn on_hotkey_queue(_: &HotkeyQueue, cx: &mut App) {
    navigate_with_root(cx, AppRoute::Queue);
}

fn on_hotkey_settings(_: &HotkeySettings, cx: &mut App) {
    navigate_with_root(cx, AppRoute::Settings);
}

fn on_hotkey_quit(_: &HotkeyQuit, cx: &mut App) {
    on_quit(&TrayQuit, cx);
}
