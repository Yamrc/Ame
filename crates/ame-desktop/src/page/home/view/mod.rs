mod load;

use std::sync::Arc;

use nekowg::{App, Context, Entity, Render, Subscription, Window, prelude::*};

use crate::app::page::PageLifecycle;
use crate::app::route::AppRoute;
use crate::app::router;
use crate::app::runtime::AppRuntime;
use crate::domain::library as library_actions;
use crate::domain::player;
use crate::domain::session as auth;
use crate::domain::settings::HomeArtistLanguage;
use crate::page::home::models::{HomePageSnapshot, HomeSessionKey};
use crate::page::home::sections::{
    OpenDailyHandler, OpenFmHandler, OpenPlaylistHandler, PlayDailyHandler, render_home_sections,
};
use crate::page::home::state::HomePageState;
use crate::page::state::freeze_page_state;

pub struct HomePageView {
    runtime: AppRuntime,
    state: Entity<HomePageState>,
    observed_session_key: HomeSessionKey,
    observed_artist_language: HomeArtistLanguage,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

impl HomePageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| HomePageState::default());
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&state, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        subscriptions.push(cx.observe(&runtime.app, |this, _, cx| {
            this.handle_app_change(cx);
        }));
        let observed_session_key = load::session_key(&runtime, cx);
        let observed_artist_language = runtime.app.read(cx).home_artist_language;
        Self {
            runtime,
            state,
            observed_session_key,
            observed_artist_language,
            active: false,
            _subscriptions: subscriptions,
        }
    }

    fn open_daily(&mut self, cx: &mut Context<Self>) {
        if auth::has_user_token(&self.runtime, cx) {
            router::navigate_route(cx, AppRoute::DailyTracks);
        } else {
            router::navigate_route(cx, AppRoute::Login);
        }
    }

    fn play_daily(&mut self, track_id: Option<i64>, cx: &mut Context<Self>) {
        if !auth::has_user_token(&self.runtime, cx) {
            router::navigate_route(cx, AppRoute::Login);
            return;
        }

        let tracks = self.state.read(cx).daily_tracks.data.clone();
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

    fn open_fm(&mut self, track: Option<library_actions::FmTrackItem>, cx: &mut Context<Self>) {
        if !auth::has_user_token(&self.runtime, cx) {
            router::navigate_route(cx, AppRoute::Login);
            return;
        }
        if let Some(track) = track {
            player::enqueue_track(&self.runtime, track.into(), true, cx);
        } else {
            router::navigate_route(cx, AppRoute::Library);
        }
    }

    fn clear_state(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.clear();
            cx.notify();
        });
    }

    fn fail_state(&mut self, error: &str, has_user_token: bool, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.recommend_playlists.clear();
            state.recommend_playlists.fail(error);
            state.recommend_artists.clear();
            state.recommend_artists.fail(error);
            state.new_albums.clear();
            state.new_albums.fail(error);
            state.toplists.clear();
            state.toplists.fail(error);
            if has_user_token {
                state.daily_tracks.clear();
                state.daily_tracks.fail(error);
                state.personal_fm.clear();
                state.personal_fm.fail(error);
            } else {
                state.daily_tracks.clear();
                state.personal_fm.clear();
            }
            cx.notify();
        });
    }
}

impl Render for HomePageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let home = self.state.read(cx).clone();
        let snapshot = HomePageSnapshot::from_states(
            &home.recommend_playlists,
            &home.recommend_artists,
            &home.new_albums,
            &home.toplists,
            &home.daily_tracks,
            &home.personal_fm,
        );
        let page = cx.entity();
        let on_open_daily: OpenDailyHandler = {
            let page = page.clone();
            Arc::new(move |cx: &mut App| {
                page.update(cx, |this, cx| this.open_daily(cx));
            })
        };
        let on_play_daily: PlayDailyHandler = {
            let page = page.clone();
            Arc::new(move |track_id: Option<i64>, cx: &mut App| {
                page.update(cx, |this, cx| this.play_daily(track_id, cx));
            })
        };
        let on_open_fm: OpenFmHandler = {
            let page = page.clone();
            Arc::new(
                move |track: Option<library_actions::FmTrackItem>, cx: &mut App| {
                    page.update(cx, |this, cx| this.open_fm(track.clone(), cx));
                },
            )
        };
        let on_open_playlist: OpenPlaylistHandler =
            Arc::new(move |playlist_id: i64, cx: &mut App| {
                page.update(cx, |_, cx| {
                    router::navigate_route(cx, AppRoute::Playlist { id: playlist_id });
                });
            });

        render_home_sections(
            snapshot,
            on_open_daily,
            on_play_daily,
            on_open_fm,
            on_open_playlist,
        )
    }
}

impl PageLifecycle for HomePageView {
    fn on_activate(&mut self, cx: &mut Context<Self>) {
        self.active = true;
        self.ensure_loaded(cx);
    }

    fn on_frozen(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        freeze_page_state(&self.state, cx);
    }
}
