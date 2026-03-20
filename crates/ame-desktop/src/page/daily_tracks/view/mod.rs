mod load;

use std::sync::Arc;

use nekowg::{Context, Entity, Render, Subscription, Window, prelude::*};

use crate::app::page::PageLifecycle;
use crate::app::runtime::AppRuntime;
use crate::domain::player;
use crate::page::daily_tracks::models::DailyTracksPageSnapshot;
use crate::page::daily_tracks::sections::{
    ReplaceDailyQueueHandler, TrackActionHandler, render_daily_tracks_page,
};
use crate::page::daily_tracks::state::DailyTracksPageState;
use crate::page::state::freeze_page_state;

pub struct DailyTracksPageView {
    runtime: AppRuntime,
    state: Entity<DailyTracksPageState>,
    last_user_id: Option<i64>,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

impl DailyTracksPageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| DailyTracksPageState::default());
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&state, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        let last_user_id = runtime.session.read(cx).auth_user_id;
        Self {
            runtime,
            state,
            last_user_id,
            active: false,
            _subscriptions: subscriptions,
        }
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
}

impl Render for DailyTracksPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = DailyTracksPageSnapshot::from_state(
            &self.state.read(cx).tracks,
            self.runtime
                .player
                .read(cx)
                .current_item()
                .map(|item| item.id),
        );
        let page = cx.entity();
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
        let on_replace_queue: ReplaceDailyQueueHandler = {
            let page = cx.entity();
            Arc::new(move |track_id, cx| {
                page.update(cx, |this, cx| this.replace_queue(track_id, cx));
            })
        };

        render_daily_tracks_page(snapshot, on_play_track, on_enqueue_track, on_replace_queue)
    }
}

impl PageLifecycle for DailyTracksPageView {
    fn on_activate(&mut self, cx: &mut Context<Self>) {
        self.active = true;
        self.ensure_loaded(cx);
    }

    fn on_frozen(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        freeze_page_state(&self.state, cx);
    }
}
