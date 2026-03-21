use std::sync::Arc;

use nekowg::{AnyElement, Context, Window, div, prelude::*, px};

use crate::app::route::AppRoute;
use crate::app::router;
use crate::component::{
    bottom_bar,
    nav_bar::{self, NavBarActions, NavBarModel},
    title_bar::{self, TitleBarActions, TitleBarModel},
};
use crate::domain::player;

use super::super::RootView;

impl RootView {
    pub(super) fn render_top_chrome(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let root_entity = cx.entity();
        let pathname = router::current_route(cx).to_path().as_ref().to_string();
        let session = self.env.session().read(cx).clone();
        let close_root = root_entity.clone();
        let top = title_bar::render(
            &TitleBarModel {
                title: "Ame".into(),
                is_maximized: window.is_maximized(),
            },
            &TitleBarActions {
                on_min: Arc::new(|window, _| window.minimize_window()),
                on_toggle_max_restore: Arc::new(|window, _| window.zoom_window()),
                on_close: Arc::new(move |window, cx| {
                    close_root.update(cx, |this, cx| this.request_window_close(window, cx));
                }),
            },
        );

        let nav_avatar = session
            .auth_user_avatar
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .cloned();

        let nav = nav_bar::render(
            &NavBarModel {
                pathname: pathname.into(),
                search_input: self.nav_search_input.clone(),
                avatar_url: nav_avatar.map(Into::into),
            },
            &NavBarActions {
                on_back: Arc::new(|_| {}),
                on_forward: Arc::new(|_| {}),
                on_home: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.navigate_to(AppRoute::Home, cx));
                    })
                },
                on_discover: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.navigate_to(AppRoute::Explore, cx));
                    })
                },
                on_library: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.navigate_to(AppRoute::Library, cx));
                    })
                },
                on_profile: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.navigate_to(AppRoute::Login, cx));
                    })
                },
            },
        );

        div()
            .absolute()
            .left(px(0.))
            .top(px(0.))
            .right(px(0.))
            .occlude()
            .backdrop_blur_xl()
            .backdrop_saturation(1.8)
            .child(top)
            .child(nav)
            .into_any_element()
    }

    pub(super) fn render_bottom_chrome(&self, cx: &mut Context<Self>) -> AnyElement {
        let root_entity = cx.entity();
        let player = self.env.player().read(cx).clone();
        let (current_song, current_artist, current_cover_url) = player
            .current_item()
            .map(|item| {
                (
                    item.name.clone(),
                    item.artist.clone(),
                    item.cover_url.clone(),
                )
            })
            .unwrap_or_else(|| ("未播放".to_string(), "未知作家".to_string(), None));
        let bottom = bottom_bar::render(
            &bottom_bar::BottomBarModel {
                current_song: current_song.into(),
                current_artist: current_artist.into(),
                current_cover_url: current_cover_url.map(Into::into),
                is_playing: player.is_playing,
                mode: player.mode,
                volume: player.volume,
                progress_slider: self.player_progress_slider.clone(),
                volume_slider: self.player_volume_slider.clone(),
            },
            &bottom_bar::BottomBarActions {
                on_prev: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.tray_previous(cx));
                    })
                },
                on_toggle: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.tray_toggle_playback(cx));
                    })
                },
                on_next: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.tray_next(cx));
                    })
                },
                on_open_queue: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.navigate_to(AppRoute::Queue, cx));
                    })
                },
                on_cycle_mode: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| {
                            player::cycle_play_mode(&this.runtime, cx);
                        });
                    })
                },
            },
        );

        div()
            .absolute()
            .left(px(0.))
            .right(px(0.))
            .bottom(px(0.))
            .occlude()
            .child(bottom)
            .into_any_element()
    }
}
