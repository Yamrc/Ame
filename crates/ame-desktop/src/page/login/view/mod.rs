mod load;

use std::rc::Rc;

use nekowg::{Context, Entity, Render, Subscription, Window, prelude::*};

use crate::app::page::{PageLifecycle, PageRetentionPolicy};
use crate::app::runtime::AppRuntime;
use crate::domain::session as auth;
use crate::page::login::models::LoginViewModel;
use crate::page::login::sections::{LoginActionHandler, render_login_page};
use crate::page::login::state::LoginPageState;

pub struct LoginPageView {
    runtime: AppRuntime,
    state: Entity<LoginPageState>,
    polling_task_active: bool,
    _subscriptions: Vec<Subscription>,
}

impl LoginPageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| LoginPageState::default());
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
        Self {
            runtime,
            state,
            polling_task_active: false,
            _subscriptions: subscriptions,
        }
    }

    fn view_model(&self, cx: &mut Context<Self>) -> LoginViewModel {
        let login = self.state.read(cx).clone();
        let session = self.runtime.session.read(cx).clone();
        let shell = self.runtime.shell.read(cx).clone();
        let auth_state = if session.auth_bundle.music_u.is_some() {
            "账号登录"
        } else if session.auth_bundle.music_a.is_some() {
            "游客登录"
        } else {
            "未登录"
        };

        LoginViewModel {
            auth_state: auth_state.to_string(),
            account_summary: session.auth_account_summary,
            qr_status: login.qr_status,
            qr_url: login.qr_url,
            qr_image: login.qr_image,
            polling: login.qr_polling,
            error: shell.error,
        }
    }
}

impl Render for LoginPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.view_model(cx);
        let page = cx.entity();
        let on_generate_qr: LoginActionHandler = Rc::new(move |cx| {
            page.update(cx, |this, cx| this.generate_login_qr(cx));
        });
        let page = cx.entity();
        let on_stop_polling: LoginActionHandler = Rc::new(move |cx| {
            page.update(cx, |this, cx| this.stop_login_qr_polling(cx));
        });
        let page = cx.entity();
        let on_guest_login: LoginActionHandler = Rc::new(move |cx| {
            page.update(cx, |this, cx| {
                auth::ensure_guest_session(&this.runtime, cx);
            });
        });
        let page = cx.entity();
        let on_refresh_login: LoginActionHandler = Rc::new(move |cx| {
            page.update(cx, |this, cx| {
                auth::refresh_login_token(&this.runtime, cx);
            });
        });

        render_login_page(
            model,
            on_generate_qr,
            on_stop_polling,
            on_guest_login,
            on_refresh_login,
        )
    }
}

impl PageLifecycle for LoginPageView {
    fn snapshot_policy(&self) -> PageRetentionPolicy {
        PageRetentionPolicy::KeepAlive
    }

    fn release_view_resources(&mut self, cx: &mut Context<Self>) {
        self.stop_login_qr_polling(cx);
        self.polling_task_active = false;
    }
}
