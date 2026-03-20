use nekowg::Context;
use tracing::debug;

use crate::domain::settings::HomeArtistLanguage;
use crate::page::home::models::{HomeLoadResult, HomeSessionKey};
use crate::page::home::service::{
    fetch_home_payload, session_key as service_session_key, session_key_from_session,
};
use crate::page::state::DataSource;

use super::HomePageView;

pub(super) fn session_key(
    runtime: &crate::app::runtime::AppRuntime,
    cx: &Context<HomePageView>,
) -> HomeSessionKey {
    service_session_key(runtime, cx)
}

impl HomePageView {
    pub(super) fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(false, cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(true, cx);
    }

    pub(super) fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let key = service_session_key(&self.runtime, cx);
        let changed = self.observed_session_key != key;
        self.observed_session_key = key;

        if changed {
            self.clear_state(cx);
        }

        if !self.active {
            return;
        }

        if changed {
            self.reload(cx);
        } else {
            cx.notify();
        }
    }

    pub(super) fn handle_app_change(&mut self, cx: &mut Context<Self>) {
        let language = self.runtime.app.read(cx).home_artist_language;
        let changed = self.observed_artist_language != language;
        self.observed_artist_language = language;

        if changed {
            self.clear_state(cx);
        }

        if !self.active {
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
        let key = session_key_from_session(&session);
        if !key.has_guest_token {
            self.clear_state(cx);
            return;
        }

        let source = if key.has_user_token {
            DataSource::User
        } else {
            DataSource::Guest
        };
        let state = self.state.read(cx).clone();
        if !force {
            if state.recommend_playlists.loading {
                return;
            }
            if state.recommend_playlists.source == source
                && state.recommend_playlists.fetched_at_ms.is_some()
            {
                return;
            }
        }

        let Some(cookie) = crate::domain::session::build_cookie_header(&session.auth_bundle) else {
            self.fail_state("缺少鉴权凭据", key.has_user_token, cx);
            return;
        };
        let artist_language = self.runtime.app.read(cx).home_artist_language;

        self.state.update(cx, |state, cx| {
            state.recommend_playlists.begin(source);
            state.recommend_artists.begin(source);
            state.new_albums.begin(source);
            state.toplists.begin(source);
            if key.has_user_token {
                state.daily_tracks.begin(DataSource::User);
                state.personal_fm.begin(DataSource::User);
            } else {
                state.daily_tracks.clear();
                state.personal_fm.clear();
            }
            cx.notify();
        });

        let page = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(
                    async move { fetch_home_payload(&cookie, key.has_user_token, artist_language) },
                )
                .await;
            if let Err(err) = page.update(cx, |this, cx| {
                this.apply_home_load(key, artist_language, result, cx)
            }) {
                debug!("home page load dropped before apply: {err}");
            }
        })
        .detach();
    }

    fn apply_home_load(
        &mut self,
        key: HomeSessionKey,
        artist_language: HomeArtistLanguage,
        result: Result<HomeLoadResult, String>,
        cx: &mut Context<Self>,
    ) {
        if service_session_key(&self.runtime, cx) != key
            || self.runtime.app.read(cx).home_artist_language != artist_language
        {
            return;
        }

        self.state.update(cx, |state, cx| {
            match result {
                Ok(result) => {
                    state
                        .recommend_playlists
                        .succeed(result.recommend_playlists, Some(result.fetched_at_ms));
                    state
                        .recommend_artists
                        .succeed(result.recommend_artists, Some(result.fetched_at_ms));
                    state
                        .new_albums
                        .succeed(result.new_albums, Some(result.fetched_at_ms));
                    state
                        .toplists
                        .succeed(result.toplists, Some(result.fetched_at_ms));
                    if key.has_user_token {
                        state
                            .daily_tracks
                            .succeed(result.daily_tracks, Some(result.fetched_at_ms));
                        state
                            .personal_fm
                            .succeed(result.personal_fm, Some(result.fetched_at_ms));
                    } else {
                        state.daily_tracks.clear();
                        state.personal_fm.clear();
                    }
                }
                Err(err) => {
                    state.recommend_playlists.clear();
                    state.recommend_playlists.fail(err.clone());
                    state.recommend_artists.clear();
                    state.recommend_artists.fail(err.clone());
                    state.new_albums.clear();
                    state.new_albums.fail(err.clone());
                    state.toplists.clear();
                    state.toplists.fail(err.clone());
                    if key.has_user_token {
                        state.daily_tracks.clear();
                        state.daily_tracks.fail(err.clone());
                        state.personal_fm.clear();
                        state.personal_fm.fail(err);
                    } else {
                        state.daily_tracks.clear();
                        state.personal_fm.clear();
                    }
                }
            }
            cx.notify();
        });
    }
}
