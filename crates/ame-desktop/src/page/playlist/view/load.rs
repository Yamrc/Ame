use nekowg::Context;
use tracing::{debug, warn};

use crate::domain::cache::CacheLookup;
use crate::domain::session as auth;
use crate::page::state::DataSource;

use super::PlaylistPageView;
use crate::page::playlist::models::{PlaylistPage, SessionLoadKey};
use crate::page::playlist::service::{
    fetch_playlist_page_payload, now_millis, read_playlist_page_cache, session_load_key,
    store_playlist_page,
};

impl PlaylistPageView {
    pub(super) fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let session_key = session_load_key(&self.runtime, cx);
        if self.last_session_key == session_key {
            return;
        }
        self.last_session_key = session_key;
        if !self.active {
            self.state.update(cx, |state, cx| {
                state.page.clear();
                cx.notify();
            });
        }
        if self.active {
            self.ensure_loaded(cx);
        }
    }

    pub(super) fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        if self.playlist_id <= 0 {
            self.state.update(cx, |state, cx| {
                state.page.fail("Invalid playlist ID");
                cx.notify();
            });
            return;
        }
        self.load_playlist(cx);
    }

    fn load_playlist(&mut self, cx: &mut Context<Self>) {
        let session = self.runtime.session.read(cx).clone();
        let session_key = session_load_key(&self.runtime, cx);
        let source = if session_key.1 {
            DataSource::User
        } else {
            DataSource::Guest
        };
        let state = self.state.read(cx).clone();
        if state.page.loading {
            return;
        }
        let mut used_stale_cache = false;

        match read_playlist_page_cache(&self.runtime, self.playlist_id, session.auth_user_id) {
            Ok(CacheLookup::Fresh(cached)) => {
                self.state.update(cx, |state, cx| {
                    state
                        .page
                        .succeed(Some(cached.value), Some(cached.fetched_at_ms));
                    state.page.source = source;
                    cx.notify();
                });
                return;
            }
            Ok(CacheLookup::Stale(cached)) => {
                self.state.update(cx, |state, cx| {
                    state
                        .page
                        .succeed(Some(cached.value), Some(cached.fetched_at_ms));
                    state.page.source = source;
                    state.page.revalidate();
                    cx.notify();
                });
                used_stale_cache = true;
            }
            Ok(CacheLookup::Miss) => {}
            Err(err) => {
                warn!(error = %err, "playlist cache read failed");
            }
        }

        let Some(cookie) = auth::build_cookie_header(&session.auth_bundle) else {
            self.state.update(cx, |state, cx| {
                state
                    .page
                    .fail_preserving_cached("Missing auth credentials");
                cx.notify();
            });
            return;
        };

        if !used_stale_cache {
            self.state.update(cx, |state, cx| {
                state.page.begin(source);
                cx.notify();
            });
        }

        let page = cx.entity().downgrade();
        let playlist_id = self.playlist_id;
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_playlist_page_payload(playlist_id, &cookie) })
                .await;
            if let Err(err) = page.update(cx, |this, cx| {
                this.apply_playlist_result(session_key, result, cx);
            }) {
                debug!("playlist page load dropped before apply: {err}");
            }
        })
        .detach();
    }

    fn apply_playlist_result(
        &mut self,
        session_key: SessionLoadKey,
        result: Result<PlaylistPage, String>,
        cx: &mut Context<Self>,
    ) {
        if session_load_key(&self.runtime, cx) != session_key {
            return;
        }

        match result {
            Ok(page) => {
                let fetched_at_ms =
                    store_playlist_page(&self.runtime, self.playlist_id, session_key.0, &page)
                        .unwrap_or_else(|_| now_millis());
                self.state.update(cx, |state, cx| {
                    state.page.succeed(Some(page), Some(fetched_at_ms));
                    cx.notify();
                });
            }
            Err(err) => {
                self.state.update(cx, |state, cx| {
                    state.page.fail_preserving_cached(err);
                    cx.notify();
                });
            }
        }
    }
}
