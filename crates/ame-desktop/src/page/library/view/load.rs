use nekowg::Context;

use crate::domain::session as auth_actions;
use crate::page::library::models::LibraryLoadResult;
use crate::page::library::service::fetch_library_payload;
use crate::page::state::DataSource;

use super::LibraryPageView;

impl LibraryPageView {
    pub(super) fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(false, cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(true, cx);
    }

    pub(super) fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let user_id = self.runtime.session.read(cx).auth_user_id;
        let changed = self.observed_user_id != user_id;
        self.observed_user_id = user_id;

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
        let Some(user_id) = session.auth_user_id else {
            self.clear_state(cx);
            return;
        };

        let mut state = self.state.read(cx).clone();
        if !force {
            if state.playlists.loading {
                return;
            }
            if state.playlists.fetched_at_ms.is_some() {
                return;
            }
        }

        let Some(cookie) = session
            .auth_bundle
            .music_u
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .and_then(|_| auth_actions::build_cookie_header(&session.auth_bundle))
        else {
            state.playlists.clear();
            state.playlists.fail("缺少鉴权凭据");
            state.liked_tracks.clear();
            state.liked_lyric_lines.clear();
            self.state.update(cx, |page_state, cx| {
                *page_state = state;
                cx.notify();
            });
            return;
        };

        state.playlists.begin(DataSource::User);
        state.liked_tracks.begin(DataSource::User);
        state.liked_lyric_lines.clear();
        self.state.update(cx, |page_state, cx| {
            *page_state = state;
            cx.notify();
        });

        let page = cx.entity();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_library_payload(user_id, &cookie) })
                .await;

            page.update(cx, |this, cx| {
                this.apply_loaded_library(user_id, result, cx);
            });
        })
        .detach();
    }

    fn apply_loaded_library(
        &mut self,
        user_id: i64,
        result: Result<LibraryLoadResult, String>,
        cx: &mut Context<Self>,
    ) {
        if self.runtime.session.read(cx).auth_user_id != Some(user_id) {
            return;
        }

        self.state.update(cx, |library, cx| {
            match result {
                Ok(result) => {
                    library
                        .playlists
                        .succeed(result.playlists, Some(result.fetched_at_ms));
                    library
                        .liked_tracks
                        .succeed(result.liked_tracks, Some(result.fetched_at_ms));
                    library.liked_lyric_lines = result.liked_lyric_lines;
                }
                Err(err) => {
                    library.playlists.clear();
                    library.playlists.fail(err.clone());
                    library.liked_tracks.clear();
                    library.liked_tracks.fail(err);
                    library.liked_lyric_lines.clear();
                }
            }
            cx.notify();
        });
    }
}
