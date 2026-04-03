use std::time::{Duration, Instant};

use nekowg::{Context, Entity};

use crate::app::route::AppRoute;
use crate::app::router::{self, RouterState};
use crate::component::{
    input,
    slider::{SliderEvent, SliderState},
};
use crate::domain::session as auth;
use crate::domain::{favorites, player};

use super::RootView;

const PLAYING_UI_NOTIFY_INTERVAL: Duration = Duration::from_millis(100);

impl RootView {
    pub(super) fn setup_subscriptions(&mut self, cx: &mut Context<Self>) {
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &self.nav_search_input,
            |this, _, event: &input::InputEvent, cx| match event {
                input::InputEvent::Change(text) => {
                    this.env
                        .app()
                        .update(cx, |app, _| app.set_search_query(text.to_string()));
                }
                input::InputEvent::Submit(text) => {
                    let text = text.to_string();
                    this.env
                        .app()
                        .update(cx, |app, _| app.set_search_query(text.clone()));
                    let query = text.trim().to_string();
                    if query.is_empty() {
                        router::navigate_route(cx, AppRoute::Search);
                    } else {
                        router::navigate_route(cx, AppRoute::SearchOverview { query });
                    }
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &self.player_volume_slider,
            |this, _: Entity<SliderState>, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = *event {
                    player::set_volume_absolute(&this.runtime, value, cx);
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &self.player_progress_slider,
            |this, _: Entity<SliderState>, event: &SliderEvent, cx| match *event {
                SliderEvent::Change(value) => {
                    player::preview_seek_ratio(&this.runtime, value, cx);
                }
                SliderEvent::Commit(value) => {
                    player::commit_seek_ratio(&this.runtime, value, cx);
                }
            },
        ));
        subscriptions.push(cx.observe_global::<RouterState>(|this, cx| {
            this.sync_route(cx);
        }));
        subscriptions.push(cx.observe(&self.runtime.session, |this, _, cx| {
            favorites::sync_session(&this.runtime, cx);
            cx.notify();
        }));
        subscriptions.push(cx.observe(&self.runtime.favorites, |_, _, cx| {
            cx.notify();
        }));
        self._subscriptions = subscriptions;
    }

    pub(super) fn prime_session(&mut self, cx: &mut Context<Self>) {
        let session = self.env.session().read(cx).clone();
        if session.auth_bundle.music_u.is_none() && session.auth_bundle.music_a.is_none() {
            auth::ensure_guest_session(&self.runtime, cx);
        } else {
            auth::refresh_login_summary(&self.runtime, cx);
        }
        favorites::sync_session(&self.runtime, cx);
    }

    pub(super) fn spawn_runtime_tick(&mut self, tick_ms: u64, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(tick_ms))
                    .await;
                let updated = this.update(cx, |this, cx| {
                    let now = Instant::now();
                    let mut should_notify = false;

                    player::sync_audio_bridge(&this.runtime, cx);
                    player::persist_progress_by_interval(
                        &this.runtime,
                        &mut this.last_player_progress_persist_at,
                        now,
                        cx,
                    );
                    if this.main_scroll.tick(&this.main_scroll_config) {
                        should_notify = true;
                    }
                    let current_scroll = this.main_scroll.handle.offset().y;
                    this.page_host
                        .update(cx, |host, _| host.sync_active_scroll(current_scroll));
                    this.sync_player_controls(cx);
                    if this.env.player().read(cx).is_playing
                        && now.duration_since(this.last_progress_ui_notify_at)
                            >= PLAYING_UI_NOTIFY_INTERVAL
                    {
                        this.last_progress_ui_notify_at = now;
                        should_notify = true;
                    }
                    if should_notify {
                        cx.notify();
                    }
                });
                if updated.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    pub(super) fn sync_player_controls(&mut self, cx: &mut Context<Self>) {
        let player_snapshot = self.env.player().read(cx).clone();
        self.player_volume_slider.update(cx, |slider, _| {
            if !slider.is_dragging() {
                slider.set_value_silent(player_snapshot.volume);
            }
        });
        self.player_progress_slider.update(cx, |slider, _| {
            if !slider.is_dragging() {
                slider.set_value_silent(player_snapshot.progress_ratio());
            }
        });
    }

    pub(super) fn sync_route(&mut self, cx: &mut Context<Self>) {
        let current_scroll = self.main_scroll.handle.offset().y;
        let restore_scroll = self.page_host.update(cx, |host, cx| {
            host.sync_active_scroll(current_scroll);
            host.handle_route_change(cx);
            host.take_pending_scroll_restore()
        });
        if let Some(target) = restore_scroll {
            self.main_scroll.jump_to(target);
            cx.notify();
        }
    }
}
