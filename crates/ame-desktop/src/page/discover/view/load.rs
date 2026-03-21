use nekowg::Context;
use tracing::debug;

use crate::domain::cache::CacheLookup;
use crate::domain::session as auth_actions;
use crate::page::discover::models::DiscoverLoadResult;
use crate::page::discover::service::{
    fetch_discover_payload, read_discover_payload_cache, store_discover_payload_cache,
};
use crate::page::state::DataSource;

use super::DiscoverPageView;

impl DiscoverPageView {
    pub(super) fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    pub(super) fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let has_user_token = self
            .runtime
            .session
            .read(cx)
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let changed = self.last_user_token_state != has_user_token;
        self.last_user_token_state = has_user_token;
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
        let has_user_token = session
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let source = if has_user_token {
            DataSource::User
        } else {
            DataSource::Guest
        };
        let state = self.state.read(cx).clone();
        if state.playlists.loading {
            return;
        }
        match read_discover_payload_cache(&self.runtime, session.auth_user_id, has_user_token) {
            Ok(CacheLookup::Fresh(cached)) => {
                self.apply_load_result(source, Ok(cached.value), Some(cached.fetched_at_ms), cx);
                return;
            }
            Ok(CacheLookup::Stale(cached)) => {
                self.apply_load_result(source, Ok(cached.value), Some(cached.fetched_at_ms), cx);
                self.state.update(cx, |discover, cx| {
                    discover.playlists.revalidate();
                    cx.notify();
                });
            }
            Ok(CacheLookup::Miss) => {}
            Err(err) => {
                discover_warn(err.as_str());
            }
        }

        let Some(cookie) = auth_actions::build_cookie_header(&session.auth_bundle) else {
            self.state.update(cx, |discover, cx| {
                discover
                    .playlists
                    .fail_preserving_cached("Missing auth credentials");
                cx.notify();
            });
            return;
        };

        self.state.update(cx, |discover, cx| {
            discover.playlists.begin(source);
            cx.notify();
        });

        let page = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_discover_payload(&cookie) })
                .await;
            if let Err(err) = page.update(cx, |this, cx| {
                this.apply_load_result(source, result, None, cx)
            }) {
                debug!("discover page load dropped before apply: {err}");
            }
        })
        .detach();
    }

    fn apply_load_result(
        &mut self,
        source: DataSource,
        result: Result<DiscoverLoadResult, String>,
        cached_fetched_at_ms: Option<u64>,
        cx: &mut Context<Self>,
    ) {
        self.state.update(cx, |discover, cx| {
            match result {
                Ok(result) => {
                    let fetched_at_ms = cached_fetched_at_ms.unwrap_or_else(|| {
                        store_discover_payload_cache(
                            &self.runtime,
                            self.runtime.session.read(cx).auth_user_id,
                            source == DataSource::User,
                            &result,
                        )
                        .unwrap_or(result.fetched_at_ms)
                    });
                    discover
                        .playlists
                        .succeed(result.playlists, Some(fetched_at_ms));
                }
                Err(err) => {
                    discover.playlists.fail_preserving_cached(err);
                }
            }
            discover.playlists.source = source;
            cx.notify();
        });
    }
}

fn discover_warn(error: &str) {
    tracing::warn!(error = error, "discover cache read failed");
}
