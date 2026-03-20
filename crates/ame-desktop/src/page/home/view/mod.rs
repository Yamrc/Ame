mod load;

use std::sync::Arc;

use nekowg::{App, Context, Entity, Render, Subscription, Window, prelude::*};

use crate::app::page::{PageLifecycle, PageRetentionPolicy};
use crate::app::route::AppRoute;
use crate::app::router;
use crate::app::runtime::AppRuntime;
use crate::domain::library as library_actions;
use crate::domain::player;
use crate::domain::session as auth;
use crate::domain::settings::HomeArtistLanguage;
use crate::page::home::models::{HomeArtistCard, HomePlaylistCard, HomeSessionKey};
use crate::page::home::sections::{
    HomeSectionsRender, OpenDailyHandler, OpenFmHandler, OpenPlaylistHandler, PlayDailyHandler,
    render_home_sections,
};
use crate::page::home::state::HomePageState;
use crate::page::state::freeze_page_state;

pub struct HomePageView {
    runtime: AppRuntime,
    state: Entity<HomePageState>,
    observed_session_key: HomeSessionKey,
    observed_artist_language: HomeArtistLanguage,
    heavy_resources: HomeHeavyResources,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

#[derive(Default)]
struct HomeHeavyResources {
    daily_card: HomePlaylistCard,
    daily_first_track_id: Option<i64>,
    fm_card: HomePlaylistCard,
    fm_track: Option<library_actions::FmTrackItem>,
    playlists: Vec<HomePlaylistCard>,
    artists: Vec<HomeArtistCard>,
    albums: Vec<HomePlaylistCard>,
    toplists: Vec<HomePlaylistCard>,
}

impl HomePageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| HomePageState::default());
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&state, |this, _, cx| {
            this.refresh_heavy_resources(cx);
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
            heavy_resources: HomeHeavyResources::default(),
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

    fn refresh_heavy_resources(&mut self, cx: &mut Context<Self>) {
        let home = self.state.read(cx);
        self.heavy_resources = HomeHeavyResources {
            daily_card: HomePlaylistCard {
                id: 0,
                name: "每日推荐".to_string(),
                subtitle: "根据你的口味更新".to_string(),
                cover_url: home
                    .daily_tracks
                    .data
                    .first()
                    .and_then(|track| track.cover_url.clone()),
            },
            daily_first_track_id: home.daily_tracks.data.first().map(|track| track.id),
            fm_card: home
                .personal_fm
                .data
                .as_ref()
                .map(|track| HomePlaylistCard {
                    id: track.id,
                    name: track.name.clone(),
                    subtitle: track.artists.clone(),
                    cover_url: track.cover_url.clone(),
                })
                .unwrap_or(HomePlaylistCard {
                    id: 0,
                    name: "私人 FM".to_string(),
                    subtitle: "连续播放你可能喜欢的音乐".to_string(),
                    cover_url: None,
                }),
            fm_track: home.personal_fm.data.clone(),
            playlists: home
                .recommend_playlists
                .data
                .iter()
                .take(10)
                .map(|item| HomePlaylistCard {
                    id: item.id,
                    name: item.name.clone(),
                    subtitle: item.creator_name.clone(),
                    cover_url: item.cover_url.clone(),
                })
                .collect(),
            artists: home
                .recommend_artists
                .data
                .iter()
                .take(6)
                .map(|artist| HomeArtistCard {
                    name: artist.name.clone(),
                    cover_url: artist.cover_url.clone(),
                })
                .collect(),
            albums: home
                .new_albums
                .data
                .iter()
                .take(10)
                .map(|album| HomePlaylistCard {
                    id: album.id,
                    name: album.name.clone(),
                    subtitle: album.artist_name.clone(),
                    cover_url: album.cover_url.clone(),
                })
                .collect(),
            toplists: home
                .toplists
                .data
                .iter()
                .take(10)
                .map(|list| HomePlaylistCard {
                    id: list.id,
                    name: list.name.clone(),
                    subtitle: list.update_frequency.clone(),
                    cover_url: list.cover_url.clone(),
                })
                .collect(),
        };
    }
}

impl Render for HomePageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let home = self.state.read(cx);
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
            HomeSectionsRender {
                loading: home.recommend_playlists.loading
                    || home.recommend_artists.loading
                    || home.new_albums.loading
                    || home.toplists.loading
                    || home.daily_tracks.loading
                    || home.personal_fm.loading,
                error: home
                    .recommend_playlists
                    .error
                    .as_deref()
                    .or(home.recommend_artists.error.as_deref())
                    .or(home.new_albums.error.as_deref())
                    .or(home.toplists.error.as_deref())
                    .or(home.daily_tracks.error.as_deref())
                    .or(home.personal_fm.error.as_deref()),
                daily_card: &self.heavy_resources.daily_card,
                daily_first_track_id: self.heavy_resources.daily_first_track_id,
                fm_card: &self.heavy_resources.fm_card,
                fm_track: self.heavy_resources.fm_track.as_ref(),
                playlists: &self.heavy_resources.playlists,
                artists: &self.heavy_resources.artists,
                albums: &self.heavy_resources.albums,
                toplists: &self.heavy_resources.toplists,
            },
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

    fn snapshot_policy(&self) -> PageRetentionPolicy {
        PageRetentionPolicy::SnapshotOnly
    }

    fn release_view_resources(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        self.heavy_resources = HomeHeavyResources::default();
        freeze_page_state(&self.state, cx);
    }
}
