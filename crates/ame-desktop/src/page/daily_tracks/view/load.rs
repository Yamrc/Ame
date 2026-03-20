use nekowg::Context;
use tracing::debug;

use crate::domain::library::DailyTrackItem;
use crate::domain::session as auth;
use crate::page::daily_tracks::service::{fetch_daily_tracks_payload, now_millis};
use crate::page::state::DataSource;

use super::DailyTracksPageView;

impl DailyTracksPageView {
    pub(super) fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(false, cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(true, cx);
    }

    pub(super) fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let user_id = self.runtime.session.read(cx).auth_user_id;
        let changed = self.last_user_id != user_id;
        self.last_user_id = user_id;
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
        if !auth::has_user_token(&self.runtime, cx) {
            self.clear_state(cx);
            return;
        }

        let state = self.state.read(cx).tracks.clone();
        if !force {
            if state.loading {
                return;
            }
            if state.fetched_at_ms.is_some() {
                return;
            }
        }

        let Some(cookie) = auth::build_cookie_header(&session.auth_bundle) else {
            self.state.update(cx, |state, cx| {
                state.tracks.clear();
                state.tracks.fail("缺少鉴权凭据");
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
            if let Err(err) = page.update(cx, |this, cx| this.apply_load_result(result, cx)) {
                debug!("daily tracks page load dropped before apply: {err}");
            }
        })
        .detach();
    }

    fn apply_load_result(
        &mut self,
        result: Result<Vec<DailyTrackItem>, String>,
        cx: &mut Context<Self>,
    ) {
        self.state.update(cx, |state, cx| {
            match result {
                Ok(items) => state.tracks.succeed(items, Some(now_millis())),
                Err(err) => {
                    state.tracks.clear();
                    state.tracks.fail(err);
                }
            }
            cx.notify();
        });
    }
}
