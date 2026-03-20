mod load;

use std::rc::Rc;

use nekowg::{AppContext, Context, Entity, Render, ScrollHandle, Subscription, Window, prelude::*};

use crate::app::page::PageLifecycle;
use crate::app::runtime::AppRuntime;
use crate::domain::player;
use crate::page::playlist::models::PlaylistPageSnapshot;
use crate::page::playlist::sections::{
    ReplaceQueueHandler, TrackActionHandler, render_playlist_page,
};
use crate::page::state::freeze_page_state;

use super::state::PlaylistPageState;

pub struct PlaylistPageView {
    runtime: AppRuntime,
    page_scroll_handle: ScrollHandle,
    playlist_id: i64,
    state: Entity<PlaylistPageState>,
    last_session_key: super::models::SessionLoadKey,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

impl PlaylistPageView {
    pub fn new(
        runtime: AppRuntime,
        page_scroll_handle: ScrollHandle,
        playlist_id: i64,
        cx: &mut Context<Self>,
    ) -> Self {
        let state = cx.new(|_| PlaylistPageState::default());
        let last_session_key = super::service::session_load_key(&runtime, cx);
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&state, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        subscriptions.push(cx.observe(&runtime.player, |_, _, cx| {
            cx.notify();
        }));
        Self {
            runtime,
            page_scroll_handle,
            playlist_id,
            state,
            last_session_key,
            active: false,
            _subscriptions: subscriptions,
        }
    }

    fn replace_queue_from_current_playlist(&mut self, cx: &mut Context<Self>) {
        let Some(page) = self.state.read(cx).page.data.clone() else {
            return;
        };
        let tracks = page
            .tracks
            .into_iter()
            .map(player::QueueTrackInput::from)
            .collect::<Vec<_>>();
        player::replace_queue(&self.runtime, tracks, 0, cx);
    }
}

impl Render for PlaylistPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = PlaylistPageSnapshot::from_state(
            self.playlist_id,
            &self.state.read(cx).page,
            self.runtime
                .player
                .read(cx)
                .current_item()
                .map(|item| item.id),
        );
        let page = cx.entity();
        let on_play_track: TrackActionHandler = {
            let page = page.clone();
            Rc::new(move |track, cx| {
                let input = player::QueueTrackInput::from(track);
                page.update(cx, |this, cx| {
                    player::enqueue_track(&this.runtime, input, true, cx);
                });
            })
        };
        let on_enqueue_track: TrackActionHandler = {
            let page = page.clone();
            Rc::new(move |track, cx| {
                let input = player::QueueTrackInput::from(track);
                page.update(cx, |this, cx| {
                    player::enqueue_track(&this.runtime, input, false, cx);
                });
            })
        };
        let on_replace_queue: ReplaceQueueHandler = {
            let page = cx.entity();
            Rc::new(move |_playlist_id, cx| {
                page.update(cx, |this, cx| this.replace_queue_from_current_playlist(cx));
            })
        };

        render_playlist_page(
            snapshot,
            &self.page_scroll_handle,
            on_play_track,
            on_enqueue_track,
            on_replace_queue,
        )
    }
}

impl PageLifecycle for PlaylistPageView {
    fn on_activate(&mut self, cx: &mut Context<Self>) {
        self.active = true;
        self.ensure_loaded(cx);
    }

    fn on_frozen(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        freeze_page_state(&self.state, cx);
    }
}
