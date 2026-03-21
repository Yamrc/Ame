use nekowg::Context;
use tracing::debug;

use crate::domain::cache::CacheLookup;
use crate::domain::library::DailyTrackItem;
use crate::domain::session as auth;
use crate::page::daily_tracks::service::{
    fetch_daily_tracks_payload, now_millis, read_daily_tracks_cache, store_daily_tracks_cache,
};
use crate::page::state::DataSource;

use super::DailyTracksPageView;

impl DailyTracksPageView {
    pub(super) fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    pub(super) fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let user_id = self.runtime.session.read(cx).auth_user_id;
        let changed = self.last_user_id != user_id;
        self.last_user_id = user_id;
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
                state.tracks.fail_preserving_cached("Missing user identity");
                cx.notify();
            });
            return;
        };
        if !auth::has_user_token(&self.runtime, cx) {
            self.state.update(cx, |state, cx| {
                state
                    .tracks
                    .fail_preserving_cached("Missing auth credentials");
                cx.notify();
            });
            return;
        }

        let state = self.state.read(cx).tracks.clone();
        if state.loading {
            return;
        }
        match read_daily_tracks_cache(&self.runtime, user_id) {
            Ok(CacheLookup::Fresh(cached)) => {
                self.apply_load_result(Ok(cached.value), Some(cached.fetched_at_ms), cx);
                return;
            }
            Ok(CacheLookup::Stale(cached)) => {
                self.apply_load_result(Ok(cached.value), Some(cached.fetched_at_ms), cx);
                self.state.update(cx, |state, cx| {
                    state.tracks.revalidate();
                    cx.notify();
                });
            }
            Ok(CacheLookup::Miss) => {}
            Err(err) => {
                tracing::warn!(error = %err, "daily tracks cache read failed");
            }
        }

        let Some(cookie) = auth::build_cookie_header(&session.auth_bundle) else {
            self.state.update(cx, |state, cx| {
                state
                    .tracks
                    .fail_preserving_cached("Missing auth credentials");
                cx.notify();
            });
            return;
        };

        self.state.update(cx, |state, cx| {
            state.tracks.begin(DataSource::User);
            cx.notify();
        });

        let page = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_daily_tracks_payload(&cookie) })
                .await;
            if let Err(err) = page.update(cx, |this, cx| this.apply_load_result(result, None, cx)) {
                debug!("daily tracks page load dropped before apply: {err}");
            }
        })
        .detach();
    }

    fn apply_load_result(
        &mut self,
        result: Result<Vec<DailyTrackItem>, String>,
        cached_fetched_at_ms: Option<u64>,
        cx: &mut Context<Self>,
    ) {
        self.state.update(cx, |state, cx| {
            match result {
                Ok(items) => {
                    let fetched_at_ms = cached_fetched_at_ms.unwrap_or_else(|| {
                        self.runtime
                            .session
                            .read(cx)
                            .auth_user_id
                            .map(|user_id| {
                                store_daily_tracks_cache(&self.runtime, user_id, &items)
                                    .unwrap_or_else(|_| now_millis())
                            })
                            .unwrap_or_else(now_millis)
                    });
                    state.tracks.succeed(items, Some(fetched_at_ms));
                }
                Err(err) => {
                    state.tracks.fail_preserving_cached(err);
                }
            }
            cx.notify();
        });
    }
}
