use nekowg::Context;
use tracing::debug;

use crate::domain::cache::CacheLookup;
use crate::domain::session as auth_actions;
use crate::page::library::models::LibraryLoadResult;
use crate::page::library::service::{
    fetch_library_payload, read_library_payload_cache, store_library_payload_cache,
};
use crate::page::state::DataSource;

use super::LibraryPageView;

impl LibraryPageView {
    pub(super) fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    pub(super) fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let user_id = self.runtime.session.read(cx).auth_user_id;
        let changed = self.observed_user_id != user_id;
        self.observed_user_id = user_id;

        if changed && !self.active {
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

    fn load(&mut self, cx: &mut Context<Self>) {
        let session = self.runtime.session.read(cx).clone();
        let Some(user_id) = session.auth_user_id else {
            self.state.update(cx, |state, cx| {
                state
                    .playlists
                    .fail_preserving_cached("Missing user identity");
                state
                    .liked_tracks
                    .fail_preserving_cached("Missing user identity");
                cx.notify();
            });
            return;
        };

        let mut used_stale_cache = false;
        if self.state.read(cx).playlists.loading {
            return;
        }
        match read_library_payload_cache(&self.runtime, user_id) {
            Ok(CacheLookup::Fresh(cached)) => {
                self.apply_loaded_library(
                    user_id,
                    Ok(cached.value),
                    Some(cached.fetched_at_ms),
                    cx,
                );
                return;
            }
            Ok(CacheLookup::Stale(cached)) => {
                self.apply_loaded_library(
                    user_id,
                    Ok(cached.value),
                    Some(cached.fetched_at_ms),
                    cx,
                );
                used_stale_cache = true;
            }
            Ok(CacheLookup::Miss) => {}
            Err(err) => {
                tracing::warn!(error = %err, "library cache read failed");
            }
        }

        let Some(cookie) = session
            .auth_bundle
            .music_u
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .and_then(|_| auth_actions::build_cookie_header(&session.auth_bundle))
        else {
            self.state.update(cx, |state, cx| {
                state
                    .playlists
                    .fail_preserving_cached("Missing auth credentials");
                state
                    .liked_tracks
                    .fail_preserving_cached("Missing auth credentials");
                if !state.liked_tracks.has_cached_value() {
                    state.liked_lyric_lines.clear();
                }
                cx.notify();
            });
            return;
        };

        if used_stale_cache {
            self.state.update(cx, |state, cx| {
                state.playlists.revalidate();
                state.playlists.source = DataSource::User;
                state.liked_tracks.revalidate();
                state.liked_tracks.source = DataSource::User;
                cx.notify();
            });
        } else {
            self.state.update(cx, |state, cx| {
                state.playlists.begin(DataSource::User);
                state.liked_tracks.begin(DataSource::User);
                state.liked_lyric_lines.clear();
                cx.notify();
            });
        }

        let page = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_library_payload(user_id, &cookie) })
                .await;

            if let Err(err) = page.update(cx, |this, cx| {
                this.apply_loaded_library(user_id, result, None, cx);
            }) {
                debug!("library page load dropped before apply: {err}");
            }
        })
        .detach();
    }

    fn apply_loaded_library(
        &mut self,
        user_id: i64,
        result: Result<LibraryLoadResult, String>,
        cached_fetched_at_ms: Option<u64>,
        cx: &mut Context<Self>,
    ) {
        if self.runtime.session.read(cx).auth_user_id != Some(user_id) {
            return;
        }

        self.state.update(cx, |library, cx| {
            match result {
                Ok(result) => {
                    let fetched_at_ms = cached_fetched_at_ms.unwrap_or_else(|| {
                        store_library_payload_cache(&self.runtime, user_id, &result)
                            .unwrap_or(result.fetched_at_ms)
                    });
                    library
                        .playlists
                        .succeed(result.playlists, Some(fetched_at_ms));
                    library
                        .liked_tracks
                        .succeed(result.liked_tracks, Some(fetched_at_ms));
                    library.liked_lyric_lines = result.liked_lyric_lines;
                }
                Err(err) => {
                    library.playlists.fail_preserving_cached(err.clone());
                    let had_cached_tracks = library.liked_tracks.has_cached_value();
                    library.liked_tracks.fail_preserving_cached(err);
                    if !had_cached_tracks {
                        library.liked_lyric_lines.clear();
                    }
                }
            }
            cx.notify();
        });
    }
}
