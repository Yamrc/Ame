use std::sync::Arc;

use nekowg::{
    App, Context, Entity, FontWeight, Render, Subscription, Window, div, prelude::*, px, rgb,
};

use crate::action::library_actions::DailyTrackItem;
use crate::component::theme;
use crate::entity::pages::DataState;
use crate::entity::player_controller::{PlayerController, QueueTrackInput};
use crate::entity::runtime::AppRuntime;
use crate::entity::services::{auth, pages};
use crate::router::{self, RouterState};
use crate::view::common;
use crate::view::playlist::{self, PlaylistTrackRow};

type TrackActionHandler = Arc<dyn Fn(PlaylistTrackRow, &mut App)>;
type ReplaceDailyQueueHandler = Arc<dyn Fn(Option<i64>, &mut App)>;

#[derive(Debug, Clone)]
pub struct DailyTracksPageSnapshot {
    pub loading: bool,
    pub error: Option<String>,
    pub tracks: Vec<PlaylistTrackRow>,
    pub first_track_id: Option<i64>,
    pub current_playing_track_id: Option<i64>,
}

impl DailyTracksPageSnapshot {
    pub fn from_state(
        state: &DataState<Vec<DailyTrackItem>>,
        current_playing_track_id: Option<i64>,
    ) -> Self {
        Self {
            loading: state.loading,
            error: state.error.clone(),
            tracks: state
                .data
                .iter()
                .map(|track| PlaylistTrackRow {
                    id: track.id,
                    name: track.name.clone(),
                    alias: track.alias.clone(),
                    artists: track.artists.clone(),
                    album: track.album.clone(),
                    duration_ms: track.duration_ms,
                    cover_url: track.cover_url.clone(),
                })
                .collect(),
            first_track_id: state.data.first().map(|track| track.id),
            current_playing_track_id,
        }
    }
}

pub struct DailyTracksPageView {
    runtime: AppRuntime,
    player_controller: Entity<PlayerController>,
    last_user_id: Option<i64>,
    _subscriptions: Vec<Subscription>,
}

impl DailyTracksPageView {
    pub fn new(
        runtime: AppRuntime,
        player_controller: Entity<PlayerController>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe_global::<RouterState>(|this, cx| {
            if this.is_active(cx) {
                this.ensure_loaded(cx);
            }
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        let last_user_id = runtime.session.read(cx).auth_user_id;
        let mut this = Self {
            runtime,
            player_controller,
            last_user_id,
            _subscriptions: subscriptions,
        };
        if this.is_active(cx) {
            this.ensure_loaded(cx);
        }
        this
    }

    fn is_active(&self, cx: &mut Context<Self>) -> bool {
        router::current_path(cx).as_ref() == "/daily/songs"
    }

    fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(false, cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(true, cx);
    }

    fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let user_id = self.runtime.session.read(cx).auth_user_id;
        let changed = self.last_user_id != user_id;
        self.last_user_id = user_id;
        if !self.is_active(cx) {
            return;
        }
        if changed {
            self.reload(cx);
        } else {
            cx.notify();
        }
    }

    fn load(&mut self, force: bool, cx: &mut Context<Self>) {
        let session = self.runtime.session.read(cx).clone();
        if !auth::has_user_token(&self.runtime, cx) {
            self.runtime.home.update(cx, |home, cx| {
                home.daily_tracks.clear();
                cx.notify();
            });
            return;
        }

        let state = self.runtime.home.read(cx).daily_tracks.clone();
        if !force {
            if state.loading {
                return;
            }
            if state.fetched_at_ms.is_some() {
                return;
            }
        }

        let Some(cookie) = crate::action::auth_actions::build_cookie_header(&session.auth_bundle)
        else {
            self.runtime.home.update(cx, |home, cx| {
                home.daily_tracks.fail("缺少鉴权凭据");
                cx.notify();
            });
            return;
        };

        self.runtime.home.update(cx, |home, cx| {
            home.daily_tracks
                .begin(crate::entity::pages::DataSource::User);
            cx.notify();
        });

        let page = cx.entity();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { pages::fetch_daily_tracks_payload(&cookie) })
                .await;
            page.update(cx, |this, cx| this.apply_load_result(result, cx));
        })
        .detach();
    }

    fn apply_load_result(
        &mut self,
        result: Result<Vec<DailyTrackItem>, String>,
        cx: &mut Context<Self>,
    ) {
        self.runtime.home.update(cx, |home, cx| {
            match result {
                Ok(items) => home.daily_tracks.succeed(items, Some(now_millis())),
                Err(err) => {
                    home.daily_tracks.clear();
                    home.daily_tracks.fail(err);
                }
            }
            cx.notify();
        });
    }

    fn replace_queue(&mut self, track_id: Option<i64>, cx: &mut Context<Self>) {
        let tracks = self.runtime.home.read(cx).daily_tracks.data.clone();
        if tracks.is_empty() {
            return;
        }
        let start_index = track_id
            .and_then(|track_id| tracks.iter().position(|track| track.id == track_id))
            .unwrap_or(0);
        let queue = tracks
            .into_iter()
            .map(QueueTrackInput::from)
            .collect::<Vec<_>>();
        self.player_controller.update(cx, |player, cx| {
            player.replace_queue(queue, start_index, cx)
        });
    }
}

impl Render for DailyTracksPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = DailyTracksPageSnapshot::from_state(
            &self.runtime.home.read(cx).daily_tracks,
            self.runtime
                .player
                .read(cx)
                .current_item()
                .map(|item| item.id),
        );
        let on_play_track: TrackActionHandler = {
            let player_controller = self.player_controller.clone();
            Arc::new(move |track, cx| {
                player_controller.update(cx, |this, cx| {
                    this.enqueue_track(track.into(), true, cx);
                });
            })
        };
        let on_enqueue_track: TrackActionHandler = {
            let player_controller = self.player_controller.clone();
            Arc::new(move |track, cx| {
                player_controller.update(cx, |this, cx| {
                    this.enqueue_track(track.into(), false, cx);
                });
            })
        };
        let on_replace_queue: ReplaceDailyQueueHandler = {
            let page = cx.entity();
            Arc::new(move |track_id, cx| {
                page.update(cx, |this, cx| this.replace_queue(track_id, cx));
            })
        };
        let rows = snapshot
            .tracks
            .into_iter()
            .map(|track| {
                let is_playing = snapshot.current_playing_track_id == Some(track.id);
                let play_track = track.clone();
                let queue_track = track.clone();
                let on_play_track = on_play_track.clone();
                let on_enqueue_track = on_enqueue_track.clone();
                playlist::track_row(
                    track,
                    is_playing,
                    move |cx| on_play_track(play_track.clone(), cx),
                    move |cx| on_enqueue_track(queue_track.clone(), cx),
                )
            })
            .collect::<Vec<_>>();
        let action = snapshot.first_track_id.map(|track_id| {
            let on_replace_queue = on_replace_queue.clone();
            crate::component::button::primary_pill("替换队列并播放")
                .on_mouse_down(nekowg::MouseButton::Left, move |_, _, cx| {
                    on_replace_queue(Some(track_id), cx);
                })
                .into_any_element()
        });
        let status = common::status_banner(
            snapshot.loading,
            snapshot.error.as_deref(),
            "加载中...",
            "加载失败",
        );
        let list = if rows.is_empty() {
            common::empty_card("暂无歌曲")
        } else {
            common::stacked_rows(rows, px(8.))
        };
        let header_content = div()
            .flex()
            .flex_col()
            .child(
                div()
                    .text_size(px(42.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                    .child("每日歌曲推荐"),
            )
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(rgb(theme::COLOR_SECONDARY))
                    .child("根据你的音乐口味生成 · 每天 6:00 更新"),
            );
        let header = if let Some(action) = action {
            div()
                .w_full()
                .flex()
                .items_end()
                .justify_between()
                .child(header_content)
                .child(action)
        } else {
            header_content
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(28.))
            .gap_4()
            .child(header)
            .child(status)
            .child(list)
    }
}

fn now_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
