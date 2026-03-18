use std::rc::Rc;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use nekowg::{
    AnyElement, App, Context, Entity, FontWeight, ListSizingBehavior, Render, ScrollHandle,
    Subscription, Window, div, prelude::*, px, rgb,
};
use serde::{Deserialize, Serialize};

use crate::component::theme;
use crate::component::track_item::{self, TrackItemActions, TrackItemProps};
use crate::component::{button, virtual_list};
use crate::entity::pages::DataState;
use crate::entity::player_controller::{PlayerController, QueueTrackInput};
use crate::entity::runtime::AppRuntime;
use crate::entity::services::pages;
use crate::router::{self, RouterState, use_params};
use crate::view::common;
use std::collections::HashMap;

type TrackActionHandler = Rc<dyn Fn(PlaylistTrackRow, &mut App)>;
type ReplaceQueueHandler = Rc<dyn Fn(i64, &mut App)>;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistTrackRow {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistPage {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub tracks: Vec<PlaylistTrackRow>,
}

#[derive(Debug, Clone)]
pub struct PlaylistPageSnapshot {
    pub playlist_id: String,
    pub playlist_id_num: i64,
    pub loading: bool,
    pub error: Option<String>,
    pub playlist: Option<PlaylistPage>,
    pub current_playing_track_id: Option<i64>,
}

impl PlaylistPageSnapshot {
    pub fn from_state(
        playlist_id: String,
        state: &DataState<HashMap<i64, PlaylistPage>>,
        current_playing_track_id: Option<i64>,
    ) -> Self {
        let playlist_id_num = playlist_id.parse::<i64>().ok().unwrap_or_default();
        Self {
            playlist_id: playlist_id.clone(),
            playlist_id_num,
            loading: state.loading,
            error: state.error.clone(),
            playlist: state.data.get(&playlist_id_num).cloned(),
            current_playing_track_id,
        }
    }
}

pub fn track_row(
    item: PlaylistTrackRow,
    is_playing: bool,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
) -> AnyElement {
    track_item::render(
        TrackItemProps {
            id: item.id,
            title: item.name,
            alias: item.alias,
            artists: item.artists,
            album: item.album,
            duration_ms: item.duration_ms,
            cover_url: item.cover_url,
            show_cover: true,
            is_playing,
        },
        TrackItemActions {
            on_play: Some(Rc::new(on_play)),
            on_enqueue: Some(Rc::new(on_enqueue)),
            ..TrackItemActions::default()
        },
    )
}

pub struct PlaylistPageView {
    runtime: AppRuntime,
    player_controller: Entity<PlayerController>,
    page_scroll_handle: ScrollHandle,
    current_playlist_id: Option<i64>,
    _subscriptions: Vec<Subscription>,
}

impl PlaylistPageView {
    pub fn new(
        runtime: AppRuntime,
        player_controller: Entity<PlayerController>,
        page_scroll_handle: ScrollHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe_global::<RouterState>(|this, cx| {
            this.sync_route(cx);
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            if this.is_active(cx) {
                this.current_playlist_id = None;
                this.sync_route(cx);
            }
        }));
        let mut this = Self {
            runtime,
            player_controller,
            page_scroll_handle,
            current_playlist_id: None,
            _subscriptions: subscriptions,
        };
        this.sync_route(cx);
        this
    }

    fn is_active(&self, cx: &mut Context<Self>) -> bool {
        router::current_path(cx).as_ref().starts_with("/playlist/")
    }

    fn current_route_playlist_id(&self, cx: &mut Context<Self>) -> Option<i64> {
        if !self.is_active(cx) {
            return None;
        }
        use_params(cx)
            .get("id")
            .and_then(|value| value.as_ref().parse::<i64>().ok())
            .filter(|id| *id > 0)
    }

    fn sync_route(&mut self, cx: &mut Context<Self>) {
        let route_id = self.current_route_playlist_id(cx);
        if route_id.is_none() {
            self.current_playlist_id = None;
            return;
        }
        if route_id == self.current_playlist_id {
            return;
        }
        self.current_playlist_id = route_id;
        if let Some(playlist_id) = route_id {
            self.load_playlist(playlist_id, cx);
        }
    }

    fn load_playlist(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        if playlist_id <= 0 {
            return;
        }

        if self
            .runtime
            .playlist
            .read(cx)
            .pages
            .data
            .contains_key(&playlist_id)
        {
            self.runtime.playlist.update(cx, |playlist, cx| {
                playlist.pages.loading = false;
                playlist.pages.error = None;
                cx.notify();
            });
            return;
        }

        let session = self.runtime.session.read(cx).clone();
        if let Some((page, fetched_at_ms)) =
            pages::cached_playlist_page(&self.runtime, playlist_id, session.auth_user_id)
        {
            self.runtime.playlist.update(cx, |playlist, cx| {
                playlist.pages.data.insert(playlist_id, page);
                playlist.pages.loading = false;
                playlist.pages.error = None;
                playlist.pages.fetched_at_ms = Some(fetched_at_ms);
                cx.notify();
            });
            return;
        }

        let Some(cookie) = crate::action::auth_actions::build_cookie_header(&session.auth_bundle)
        else {
            self.runtime.playlist.update(cx, |playlist, cx| {
                playlist.pages.fail("缺少鉴权凭据");
                cx.notify();
            });
            return;
        };

        self.runtime.playlist.update(cx, |playlist, cx| {
            playlist
                .pages
                .begin(crate::entity::pages::DataSource::Guest);
            cx.notify();
        });

        let page = cx.entity();
        let expected_user_id = session.auth_user_id;
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { pages::fetch_playlist_page_payload(playlist_id, &cookie) })
                .await;
            page.update(cx, |this, cx| {
                this.apply_playlist_result(playlist_id, expected_user_id, result, cx);
            });
        })
        .detach();
    }

    fn apply_playlist_result(
        &mut self,
        playlist_id: i64,
        expected_user_id: Option<i64>,
        result: Result<PlaylistPage, String>,
        cx: &mut Context<Self>,
    ) {
        if self.current_route_playlist_id(cx) != Some(playlist_id)
            || self.runtime.session.read(cx).auth_user_id != expected_user_id
        {
            return;
        }

        match result {
            Ok(page) => {
                let fetched_at_ms =
                    pages::cache_playlist_page(&self.runtime, playlist_id, expected_user_id, &page)
                        .or_else(|| Some(now_millis()));
                self.runtime.playlist.update(cx, |playlist, cx| {
                    playlist.pages.data.insert(playlist_id, page);
                    playlist.pages.loading = false;
                    playlist.pages.error = None;
                    playlist.pages.fetched_at_ms = fetched_at_ms;
                    cx.notify();
                });
            }
            Err(err) => {
                self.runtime.playlist.update(cx, |playlist, cx| {
                    playlist.pages.loading = false;
                    playlist.pages.error = Some(err);
                    cx.notify();
                });
            }
        }
    }

    fn replace_queue_from_current_playlist(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        let Some(page) = self
            .runtime
            .playlist
            .read(cx)
            .pages
            .data
            .get(&playlist_id)
            .cloned()
        else {
            return;
        };
        let tracks = page
            .tracks
            .into_iter()
            .map(QueueTrackInput::from)
            .collect::<Vec<_>>();
        self.player_controller
            .update(cx, |player, cx| player.replace_queue(tracks, 0, cx));
    }
}

impl Render for PlaylistPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let playlist_id = use_params(cx)
            .get("id")
            .map(|value| value.as_ref().to_string())
            .unwrap_or_else(|| "0".to_string());
        let snapshot = PlaylistPageSnapshot::from_state(
            playlist_id,
            &self.runtime.playlist.read(cx).pages,
            self.runtime
                .player
                .read(cx)
                .current_item()
                .map(|item| item.id),
        );
        let on_play_track: TrackActionHandler = {
            let player_controller = self.player_controller.clone();
            Rc::new(move |track, cx| {
                player_controller.update(cx, |this, cx| {
                    this.enqueue_track(track.into(), true, cx);
                });
            })
        };
        let on_enqueue_track: TrackActionHandler = {
            let player_controller = self.player_controller.clone();
            Rc::new(move |track, cx| {
                player_controller.update(cx, |this, cx| {
                    this.enqueue_track(track.into(), false, cx);
                });
            })
        };
        let on_replace_queue: ReplaceQueueHandler = {
            let page = cx.entity();
            Rc::new(move |playlist_id, cx| {
                page.update(cx, |this, cx| {
                    this.replace_queue_from_current_playlist(playlist_id, cx);
                });
            })
        };
        let playlist_rows = snapshot.playlist.as_ref().and_then(|page| {
            if page.tracks.is_empty() {
                return None;
            }
            let tracks = Arc::new(page.tracks.clone());
            let heights = Arc::new(vec![px(84.); tracks.len()]);
            let current_playing_track_id = snapshot.current_playing_track_id;
            let on_play_track = on_play_track.clone();
            let on_enqueue_track = on_enqueue_track.clone();
            let list = virtual_list::v_virtual_list(
                ("playlist-tracks", page.id.unsigned_abs() as usize),
                heights,
                move |visible_range, _, _| {
                    visible_range
                        .map(|index| {
                            let track = tracks[index].clone();
                            let is_playing = current_playing_track_id == Some(track.id);
                            let play_track = track.clone();
                            let queue_track = track.clone();
                            let on_play_track = on_play_track.clone();
                            let on_enqueue_track = on_enqueue_track.clone();
                            nekowg::div().w_full().pb(px(8.)).child(track_row(
                                track,
                                is_playing,
                                move |cx| on_play_track(play_track.clone(), cx),
                                move |cx| on_enqueue_track(queue_track.clone(), cx),
                            ))
                        })
                        .collect::<Vec<_>>()
                },
            )
            .with_external_viewport_scroll(&self.page_scroll_handle)
            .with_sizing_behavior(ListSizingBehavior::Infer)
            .with_overscan(2)
            .w_full();
            Some(list.into_any_element())
        });
        let replace_queue_button = snapshot.playlist.as_ref().and_then(|page| {
            if page.tracks.is_empty() {
                return None;
            }
            let on_replace_queue = on_replace_queue.clone();
            let playlist_id_num = snapshot.playlist_id_num;
            Some(
                button::primary_pill("替换队列并播放")
                    .on_mouse_down(nekowg::MouseButton::Left, move |_, _, cx| {
                        on_replace_queue(playlist_id_num, cx);
                    })
                    .into_any_element(),
            )
        });
        let title = snapshot
            .playlist
            .as_ref()
            .map(|item| item.name.clone())
            .unwrap_or_else(|| format!("歌单 #{}", snapshot.playlist_id));
        let subtitle = snapshot
            .playlist
            .as_ref()
            .map(|item| format!("{} 首 · {}", item.track_count, item.creator_name))
            .unwrap_or_else(|| "待加载".to_string());

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(28.))
            .gap_5()
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(38.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(theme::COLOR_TEXT_DARK))
                            .child(title),
                    )
                    .child(replace_queue_button.unwrap_or_else(|| div().into_any_element())),
            )
            .child(
                div()
                    .text_size(px(16.))
                    .text_color(rgb(theme::COLOR_SECONDARY))
                    .child(subtitle),
            )
            .child(common::status_banner(
                snapshot.loading,
                snapshot.error.as_deref(),
                "加载中...",
                "加载失败",
            ))
            .child(
                div()
                    .w_full()
                    .child(if let Some(track_list) = playlist_rows {
                        track_list
                    } else {
                        common::empty_card("暂无歌曲")
                    }),
            )
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
