mod load;

use std::rc::Rc;
use std::sync::Arc;

use nekowg::{
    AppContext, Context, Entity, Render, ScrollHandle, Subscription, Window, prelude::*, px,
};

use crate::app::page::{PageLifecycle, PageRetentionPolicy};
use crate::app::runtime::AppRuntime;
use crate::domain::{favorites, player};
use crate::page::playlist::sections::{
    FavoriteTrackHandler, PlaylistFavoriteState, PlaylistListRenderCache,
    PlaylistRenderActions, ReplaceQueueHandler, TrackActionHandler, render_playlist_page,
};
use crate::page::state::freeze_page_state;

use super::state::PlaylistPageState;

pub struct PlaylistPageView {
    runtime: AppRuntime,
    page_scroll_handle: ScrollHandle,
    playlist_id: i64,
    state: Entity<PlaylistPageState>,
    last_session_key: super::models::SessionLoadKey,
    last_favorite_change_revision: u64,
    heavy_resources: Option<PlaylistListRenderCache>,
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
        let last_favorite_change_revision = runtime.favorites.read(cx).change_revision;
        let mut view = Self {
            runtime,
            page_scroll_handle,
            playlist_id,
            state,
            last_session_key,
            last_favorite_change_revision,
            heavy_resources: None,
            active: false,
            _subscriptions: Vec::new(),
        };
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
        subscriptions.push(cx.observe(&view.runtime.favorites, |this, _, cx| {
            this.handle_favorites_change(cx);
        }));
        view._subscriptions = subscriptions;
        view.refresh_heavy_resources(cx);
        view
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

    fn refresh_heavy_resources(&mut self, cx: &mut Context<Self>) {
        let page_state = self.state.read(cx);
        let current_playing_track_id = self
            .runtime
            .player
            .read(cx)
            .current_item()
            .map(|item| item.id);
        self.heavy_resources = page_state.page.data.as_ref().map(|page| {
            let tracks = Arc::new(page.tracks.clone());
            PlaylistListRenderCache {
                playlist_id: page.id,
                title: page.name.clone(),
                subtitle: format!("{} 首 · {}", page.track_count, page.creator_name),
                heights: Arc::new(vec![px(60.); tracks.len()]),
                tracks,
                current_playing_track_id,
            }
        });
    }

    fn handle_favorites_change(&mut self, cx: &mut Context<Self>) {
        let favorites = self.runtime.favorites.read(cx).clone();
        let changed = favorites.change_revision != self.last_favorite_change_revision;
        self.last_favorite_change_revision = favorites.change_revision;

        if changed && self.active && favorites.liked_playlist_id == Some(self.playlist_id) {
            self.ensure_loaded(cx);
        } else {
            cx.notify();
        }
    }
}

impl Render for PlaylistPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let playlist_state = self.state.read(cx);
        let page = cx.entity();
        let session_user_id = self.runtime.session.read(cx).auth_user_id;
        let favorites_state = self.runtime.favorites.read(cx).clone();
        let favorite_ready = favorites_state.is_ready_for(session_user_id);
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
        let on_toggle_favorite: FavoriteTrackHandler = {
            let page = page.clone();
            Rc::new(move |track_id, cx| {
                page.update(cx, |this, cx| {
                    favorites::toggle_track_like(&this.runtime, track_id, cx);
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
            self.playlist_id,
            &playlist_state.page,
            self.heavy_resources.as_ref(),
            &self.page_scroll_handle,
            PlaylistFavoriteState {
                favorites: favorites_state,
                ready: favorite_ready,
            },
            PlaylistRenderActions {
                on_play_track,
                on_enqueue_track,
                on_toggle_favorite,
                on_replace_queue,
            },
        )
    }
}

impl PageLifecycle for PlaylistPageView {
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
