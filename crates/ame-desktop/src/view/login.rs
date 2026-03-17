use std::sync::Arc;

use nekowg::{
    Context, Entity, FontWeight, Image, MouseButton, Render, Subscription, Window, div, img,
    prelude::*, px, rgb,
};

use crate::component::{button, theme};
use crate::entity::runtime::AppRuntime;
use crate::entity::session_controller::SessionController;

#[derive(Debug, Clone, Default)]
pub struct LoginViewModel {
    pub auth_state: String,
    pub account_summary: Option<String>,
    pub qr_status: Option<String>,
    pub qr_url: Option<String>,
    pub qr_image: Option<Arc<Image>>,
    pub polling: bool,
    pub error: Option<String>,
}

pub struct LoginPageView {
    runtime: AppRuntime,
    session_controller: Entity<SessionController>,
    _subscriptions: Vec<Subscription>,
}

impl LoginPageView {
    pub fn new(
        runtime: AppRuntime,
        session_controller: Entity<SessionController>,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            runtime,
            session_controller,
            _subscriptions: Vec::new(),
        }
    }

    fn view_model(&self, cx: &mut Context<Self>) -> LoginViewModel {
        let login = self.runtime.login.read(cx).clone();
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
        let session_controller = self.session_controller.clone();
        let qr_area = if let Some(qr_image) = model.qr_image.clone() {
            div()
                .w(px(280.))
                .h(px(280.))
                .rounded_lg()
                .bg(rgb(theme::COLOR_BODY_BG_DARK))
                .p_2()
                .child(img(qr_image).w_full().h_full().rounded_lg())
                .into_any_element()
        } else {
            div()
                .w(px(280.))
                .h(px(280.))
                .rounded_lg()
                .bg(rgb(theme::COLOR_BODY_BG_DARK))
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child("尚未生成二维码")
                .into_any_element()
        };

        let qr_url = model.qr_url.clone().unwrap_or_else(|| "无".to_string());
        let qr_status = model
            .qr_status
            .clone()
            .unwrap_or_else(|| "未开始".to_string());
        let account = model
            .account_summary
            .clone()
            .unwrap_or_else(|| "无".to_string());

        let error = model
            .error
            .as_ref()
            .map(|error| {
                div()
                    .w_full()
                    .rounded_lg()
                    .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
                    .px_4()
                    .py_3()
                    .text_color(rgb(theme::COLOR_SECONDARY))
                    .child(error.clone())
                    .into_any_element()
            })
            .unwrap_or_else(|| div().into_any_element());

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(32.))
            .gap_6()
            .child(
                div()
                    .text_size(px(42.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                    .child("登录"),
            )
            .child(error)
            .child(
                div()
                    .w_full()
                    .rounded_lg()
                    .bg(rgb(theme::COLOR_CARD_DARK))
                    .px_4()
                    .py_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(format!("当前状态: {}", model.auth_state))
                    .child(format!("账号信息: {account}"))
                    .child(format!("二维码状态: {qr_status}"))
                    .child(format!(
                        "轮询中: {}",
                        if model.polling { "是" } else { "否" }
                    ))
                    .child(div().text_color(rgb(theme::COLOR_SECONDARY)).child(qr_url)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap_6()
                    .child(qr_area)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(button::pill_base("生成二维码").on_mouse_down(
                                MouseButton::Left,
                                move |_, _, cx| {
                                    session_controller
                                        .update(cx, |this, cx| this.generate_login_qr(cx));
                                },
                            ))
                            .child({
                                let session_controller = self.session_controller.clone();
                                button::pill_base("停止轮询").on_mouse_down(
                                    MouseButton::Left,
                                    move |_, _, cx| {
                                        session_controller
                                            .update(cx, |this, cx| this.stop_login_qr_polling(cx));
                                    },
                                )
                            })
                            .child({
                                let session_controller = self.session_controller.clone();
                                button::primary_pill("游客登录").on_mouse_down(
                                    MouseButton::Left,
                                    move |_, _, cx| {
                                        session_controller
                                            .update(cx, |this, cx| this.ensure_guest_session(cx));
                                    },
                                )
                            })
                            .child({
                                let session_controller = self.session_controller.clone();
                                button::pill_base("刷新登录态").on_mouse_down(
                                    MouseButton::Left,
                                    move |_, _, cx| {
                                        session_controller
                                            .update(cx, |this, cx| this.refresh_login_token(cx));
                                    },
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}
