use nekowg::Context;

use crate::domain::session as auth_actions;
use crate::page::discover::models::DiscoverLoadResult;
use crate::page::discover::service::fetch_discover_payload;
use crate::page::state::DataSource;

use super::DiscoverPageView;

impl DiscoverPageView {
    pub(super) fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(false, cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(true, cx);
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
        if !force {
            if state.playlists.loading {
                return;
            }
            if state.playlists.source == source && state.playlists.fetched_at_ms.is_some() {
                return;
            }
        }

        let Some(cookie) = auth_actions::build_cookie_header(&session.auth_bundle) else {
            self.state.update(cx, |discover, cx| {
                discover.playlists.clear();
                discover.playlists.fail("缺少鉴权凭据");
                cx.notify();
            });
            return;
        };

        self.state.update(cx, |discover, cx| {
            discover.playlists.begin(source);
            cx.notify();
        });

        let page = cx.entity();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_discover_payload(&cookie) })
                .await;
            page.update(cx, |this, cx| this.apply_load_result(source, result, cx));
        })
        .detach();
    }

    fn apply_load_result(
        &mut self,
        source: DataSource,
        result: Result<DiscoverLoadResult, String>,
        cx: &mut Context<Self>,
    ) {
        self.state.update(cx, |discover, cx| {
            match result {
                Ok(result) => discover
                    .playlists
                    .succeed(result.playlists, Some(result.fetched_at_ms)),
                Err(err) => {
                    discover.playlists.clear();
                    discover.playlists.fail(err);
                }
            }
            discover.playlists.source = source;
            cx.notify();
        });
    }
}
