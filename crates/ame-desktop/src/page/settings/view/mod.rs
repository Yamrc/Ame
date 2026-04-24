mod actions;
mod load;

use std::rc::Rc;

use nekowg::{Context, Entity, Render, Subscription, Window, prelude::*};

use crate::app::page::{PageLifecycle, PageRetentionPolicy};
use crate::app::runtime::AppRuntime;
use crate::page::settings::models::{
    LastFmAccountViewModel, NeteaseAccountViewModel, SettingsViewModel,
};
use crate::page::settings::sections::{
    CloseBehaviorHandler, HomeArtistLanguageHandler, SettingsActionHandler, render_settings_page,
};
use crate::page::settings::state::SettingsPageState;

pub struct SettingsPageView {
    runtime: AppRuntime,
    state: Entity<SettingsPageState>,
    polling_task_active: bool,
    _subscriptions: Vec<Subscription>,
}

impl SettingsPageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| SettingsPageState::default());
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&state, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.session, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.shell, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.app, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.lastfm, |_, _, cx| {
            cx.notify();
        }));

        Self {
            runtime,
            state,
            polling_task_active: false,
            _subscriptions: subscriptions,
        }
    }

    fn view_model(&self, cx: &mut Context<Self>) -> SettingsViewModel {
        let state = self.state.read(cx).clone();
        let session = self.runtime.session.read(cx).clone();
        let shell = self.runtime.shell.read(cx).clone();
        let lastfm = self.runtime.lastfm.read(cx).clone();
        let netease_auth_state = if session.auth_bundle.music_u.is_some() {
            "账号登录"
        } else if session.auth_bundle.music_a.is_some() {
            "游客登录"
        } else {
            "未登录"
        };

        SettingsViewModel {
            close_behavior_label: shell.close_behavior.label().into(),
            home_artist_language_label: self
                .runtime
                .app
                .read(cx)
                .home_artist_language
                .label()
                .into(),
            netease: NeteaseAccountViewModel {
                auth_state: netease_auth_state.into(),
                account_summary: session
                    .auth_account_summary
                    .unwrap_or_else(|| "无".to_string())
                    .into(),
                qr_status: state
                    .qr_status
                    .unwrap_or_else(|| "未开始".to_string())
                    .into(),
                qr_url: state.qr_url.map(Into::into),
                qr_image: state.qr_image,
                polling: state.qr_polling,
            },
            lastfm: LastFmAccountViewModel {
                status: lastfm.status_label().into(),
                account_summary: lastfm
                    .session
                    .as_ref()
                    .and_then(|session| session.user_name.as_deref())
                    .map(|user_name| format!("@{user_name}"))
                    .unwrap_or_else(|| {
                        if lastfm.is_connected() {
                            "已保存 session key".to_string()
                        } else {
                            "未连接".to_string()
                        }
                    })
                    .into(),
                queue_summary: lastfm.queue_summary().into(),
                configured: lastfm.configured,
                connected: lastfm.is_connected(),
                auth_loading: lastfm.auth_inflight,
                error: lastfm.error.map(Into::into),
            },
            app_error: shell.error.map(Into::into),
        }
    }
}

impl Render for SettingsPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.view_model(cx);
        let page = cx.entity();
        let on_set_close_behavior: CloseBehaviorHandler = Rc::new(move |value, cx| {
            page.update(cx, |this, cx| this.set_close_behavior(value, cx));
        });
        let page = cx.entity();
        let on_set_home_artist_language: HomeArtistLanguageHandler = Rc::new(move |value, cx| {
            page.update(cx, |this, cx| this.set_home_artist_language(value, cx));
        });
        let page = cx.entity();
        let on_generate_qr: SettingsActionHandler = Rc::new(move |cx| {
            page.update(cx, |this, cx| this.generate_login_qr(cx));
        });
        let page = cx.entity();
        let on_refresh_login: SettingsActionHandler = Rc::new(move |cx| {
            page.update(cx, |this, cx| this.refresh_login(cx));
        });
        let page = cx.entity();
        let on_connect_lastfm: SettingsActionHandler = Rc::new(move |cx| {
            page.update(cx, |this, cx| this.connect_lastfm(cx));
        });
        let page = cx.entity();
        let on_disconnect_lastfm: SettingsActionHandler = Rc::new(move |cx| {
            page.update(cx, |this, cx| this.disconnect_lastfm(cx));
        });

        render_settings_page(
            model,
            on_generate_qr,
            on_refresh_login,
            on_connect_lastfm,
            on_disconnect_lastfm,
            on_set_close_behavior,
            on_set_home_artist_language,
        )
    }
}

impl PageLifecycle for SettingsPageView {
    fn snapshot_policy(&self) -> PageRetentionPolicy {
        PageRetentionPolicy::KeepAlive
    }

    fn release_view_resources(&mut self, cx: &mut Context<Self>) {
        self.stop_login_qr_polling(cx);
        self.polling_task_active = false;
    }
}
