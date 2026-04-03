mod load;

use std::sync::Arc;

use nekowg::{Context, Entity, Render, Subscription, Window, prelude::*};

use crate::app::page::{PageLifecycle, PageRetentionPolicy};
use crate::app::runtime::AppRuntime;
use crate::domain::{favorites, player};
use crate::page::daily_tracks::sections::{
    DailyTracksFavoriteState, DailyTracksRenderActions, DailyTracksRenderCache,
    FavoriteTrackHandler, ReplaceDailyQueueHandler, TrackActionHandler, render_daily_tracks_page,
};
use crate::page::daily_tracks::state::DailyTracksPageState;
use crate::page::playlist::PlaylistTrackRow;
use crate::page::state::freeze_page_state;

pub struct DailyTracksPageView {
    runtime: AppRuntime,
    state: Entity<DailyTracksPageState>,
    last_user_id: Option<i64>,
    heavy_resources: Option<DailyTracksRenderCache>,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

impl DailyTracksPageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| DailyTracksPageState::default());
        let mut view = Self {
            runtime,
            state,
            last_user_id: None,
            heavy_resources: None,
            active: false,
            _subscriptions: Vec::new(),
        };
        view.last_user_id = view.runtime.session.read(cx).auth_user_id;
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&view.state, |this, _, cx| {
            this.refresh_heavy_resources(cx);
            cx.notify();
        }));
        subscriptions.push(cx.observe(&view.runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        subscriptions.push(cx.observe(&view.runtime.player, |this, _, cx| {
            this.refresh_heavy_resources(cx);
            cx.notify();
        }));
        subscriptions.push(cx.observe(&view.runtime.favorites, |_, _, cx| {
            cx.notify();
        }));
        view._subscriptions = subscriptions;
        view.refresh_heavy_resources(cx);
        view
    }

    fn replace_queue(&mut self, track_id: Option<i64>, cx: &mut Context<Self>) {
        let tracks = self.state.read(cx).tracks.data.clone();
        if tracks.is_empty() {
            return;
        }
        let start_index = track_id
            .and_then(|target_id| tracks.iter().position(|track| track.id == target_id))
            .unwrap_or(0);
        let queue = tracks
            .into_iter()
            .map(player::QueueTrackInput::from)
            .collect::<Vec<_>>();
        player::replace_queue(&self.runtime, queue, start_index, cx);
    }

    fn clear_state(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.tracks.clear();
            cx.notify();
        });
    }

    fn refresh_heavy_resources(&mut self, cx: &mut Context<Self>) {
        let tracks_state = self.state.read(cx);
        let current_playing_track_id = self
            .runtime
            .player
            .read(cx)
            .current_item()
            .map(|item| item.id);
        let rows = Arc::new(
            tracks_state
                .tracks
                .data
                .iter()
                .cloned()
                .map(PlaylistTrackRow::from)
                .collect::<Vec<_>>(),
        );
        self.heavy_resources = Some(DailyTracksRenderCache {
            first_track_id: tracks_state.tracks.data.first().map(|track| track.id),
            current_playing_track_id,
            rows,
        });
    }
}

impl Render for DailyTracksPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tracks = self.state.read(cx);
        let page = cx.entity();
        let session_user_id = self.runtime.session.read(cx).auth_user_id;
        let favorites_state = self.runtime.favorites.read(cx).clone();
        let favorite_ready = favorites_state.is_ready_for(session_user_id);
        let on_play_track: TrackActionHandler = {
            let page = page.clone();
            Arc::new(move |track, cx| {
                let input = player::QueueTrackInput::from(track);
                page.update(cx, |this, cx| {
                    player::enqueue_track(&this.runtime, input, true, cx);
                });
            })
        };
        let on_enqueue_track: TrackActionHandler = {
            let page = page.clone();
            Arc::new(move |track, cx| {
                let input = player::QueueTrackInput::from(track);
                page.update(cx, |this, cx| {
                    player::enqueue_track(&this.runtime, input, false, cx);
                });
            })
        };
        let on_toggle_favorite: FavoriteTrackHandler = {
            let page = page.clone();
            Arc::new(move |track_id, cx| {
                page.update(cx, |this, cx| {
                    favorites::toggle_track_like(&this.runtime, track_id, cx);
                });
            })
        };
        let on_replace_queue: ReplaceDailyQueueHandler = {
            let page = cx.entity();
            Arc::new(move |track_id, cx| {
                page.update(cx, |this, cx| this.replace_queue(track_id, cx));
            })
        };

        render_daily_tracks_page(
            &tracks.tracks,
            self.heavy_resources.as_ref(),
            DailyTracksFavoriteState {
                favorites: favorites_state,
                ready: favorite_ready,
            },
            DailyTracksRenderActions {
                on_play_track,
                on_enqueue_track,
                on_toggle_favorite,
                on_replace_queue,
            },
        )
    }
}

impl PageLifecycle for DailyTracksPageView {
    fn on_activate(&mut self, cx: &mut Context<Self>) {
        self.active = true;
        self.ensure_loaded(cx);
    }

    fn snapshot_policy(&self) -> PageRetentionPolicy {
        PageRetentionPolicy::SnapshotOnly
    }

    fn release_view_resources(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        self.heavy_resources = None;
        freeze_page_state(&self.state, cx);
    }
}
