use std::sync::Arc;
use std::time::{Duration, Instant};

use ame_core::credential::AuthBundle;
use nekowg::{Context, EventEmitter, Image, ImageFormat};
use qrcode::{QrCode, render::svg};

use crate::action::auth_actions;
use crate::entity::runtime::AppRuntime;
use crate::entity::services::{auth, pages};

#[derive(Debug, Clone)]
pub enum SessionEvent {
    SummaryRefreshed,
}

impl EventEmitter<SessionEvent> for SessionController {}

pub struct SessionController {
    runtime: AppRuntime,
    guest_loading: bool,
    summary_loading: bool,
}

impl SessionController {
    pub fn new(runtime: AppRuntime, _cx: &mut Context<Self>) -> Self {
        Self {
            runtime,
            guest_loading: false,
            summary_loading: false,
        }
    }

    pub fn ensure_guest_session(&mut self, cx: &mut Context<Self>) {
        let bundle = self.runtime.session.read(cx).auth_bundle.clone();
        if self.guest_loading || bundle_has_guest_token(&bundle) {
            return;
        }

        self.guest_loading = true;
        let controller = cx.entity();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_guest_bundle(bundle) })
                .await;
            controller.update(cx, |this, cx| {
                this.apply_guest_session_result(result, cx);
            });
        })
        .detach();
    }

    pub fn refresh_login_summary(&mut self, cx: &mut Context<Self>) {
        let bundle = self.runtime.session.read(cx).auth_bundle.clone();
        if !bundle_has_user_token(&bundle) {
            self.clear_login_summary(cx);
            return;
        }

        if self.summary_loading {
            return;
        }

        self.summary_loading = true;
        let expected_music_u = bundle.music_u.clone();
        let controller = cx.entity();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { fetch_login_summary(bundle) })
                .await;
            controller.update(cx, |this, cx| {
                this.apply_login_summary_result(expected_music_u, result, cx);
            });
        })
        .detach();
    }

    pub fn refresh_login_token(&mut self, cx: &mut Context<Self>) {
        if !auth::has_user_token(&self.runtime, cx) {
            auth::push_shell_error(
                &self.runtime,
                "当前不是账号登录态，无法刷新登录令牌".to_string(),
                cx,
            );
            return;
        }

        let Some(cookie) = auth::ensure_auth_cookie(&self.runtime, auth::AuthLevel::User, cx)
        else {
            return;
        };
        match auth_actions::refresh_login_token_blocking(Some(cookie.as_str())) {
            Ok(response) => {
                auth::merge_auth_cookies(&self.runtime, &response.set_cookie, cx);
                self.refresh_login_summary(cx);
                self.runtime.login.update(cx, |login, cx| {
                    login.qr_status = Some("登录令牌已刷新".to_string());
                    cx.notify();
                });
            }
            Err(err) => {
                auth::push_shell_error(&self.runtime, format!("刷新登录令牌失败: {err}"), cx);
            }
        }
    }

    pub fn generate_login_qr(&mut self, cx: &mut Context<Self>) {
        let Some(cookie) = auth::ensure_auth_cookie(&self.runtime, auth::AuthLevel::Guest, cx)
        else {
            return;
        };
        let response = match auth_actions::fetch_login_qr_key_blocking(Some(cookie.as_str())) {
            Ok(response) => response,
            Err(err) => {
                auth::push_shell_error(&self.runtime, format!("获取二维码 key 失败: {err}"), cx);
                return;
            }
        };

        let key = response
            .body
            .unikey
            .clone()
            .filter(|value| !value.is_empty());
        let Some(key) = key else {
            auth::push_shell_error(&self.runtime, "二维码 key 为空".to_string(), cx);
            return;
        };

        let qr_url = format!("https://music.163.com/login?codekey={key}");
        let image_data = match QrCode::new(qr_url.as_bytes()) {
            Ok(code) => {
                let svg = code
                    .render::<svg::Color<'_>>()
                    .min_dimensions(280, 280)
                    .build();
                Some(Arc::new(Image::from_bytes(
                    ImageFormat::Svg,
                    svg.into_bytes(),
                )))
            }
            Err(err) => {
                auth::push_shell_error(&self.runtime, format!("渲染二维码失败: {err}"), cx);
                None
            }
        };

        self.runtime.login.update(cx, |login, cx| {
            login.qr_key = Some(key);
            login.qr_url = Some(qr_url);
            login.qr_image = image_data;
            login.qr_status = Some("801 等待扫码".to_string());
            login.qr_polling = true;
            login.qr_poll_started_at = Some(Instant::now());
            login.qr_last_polled_at = None;
            cx.notify();
        });
    }

    pub fn stop_login_qr_polling(&mut self, cx: &mut Context<Self>) {
        self.runtime.login.update(cx, |login, cx| {
            login.qr_polling = false;
            login.qr_status = Some("已停止轮询".to_string());
            cx.notify();
        });
    }

    pub fn stop_background_work(&mut self, cx: &mut Context<Self>) {
        self.runtime.login.update(cx, |login, _| {
            login.qr_polling = false;
        });
    }

    pub fn tick_qr_poll(&mut self, now: Instant, cx: &mut Context<Self>) -> bool {
        let login = self.runtime.login.read(cx).clone();
        if !login.qr_polling {
            return false;
        }

        if let Some(started_at) = login.qr_poll_started_at
            && now.duration_since(started_at) >= Duration::from_secs(120)
        {
            self.runtime.login.update(cx, |login, cx| {
                login.qr_polling = false;
                login.qr_status = Some("800 二维码过期".to_string());
                cx.notify();
            });
            return true;
        }

        if let Some(last) = login.qr_last_polled_at
            && now.duration_since(last) < Duration::from_secs(2)
        {
            return false;
        }

        let Some(key) = login.qr_key.clone() else {
            self.runtime.login.update(cx, |login, cx| {
                login.qr_polling = false;
                login.qr_status = Some("二维码 key 丢失".to_string());
                cx.notify();
            });
            return true;
        };

        self.runtime.login.update(cx, |login, _| {
            login.qr_last_polled_at = Some(now);
        });

        let Some(cookie) = auth::ensure_auth_cookie(&self.runtime, auth::AuthLevel::Guest, cx)
        else {
            self.runtime.login.update(cx, |login, cx| {
                login.qr_polling = false;
                cx.notify();
            });
            return true;
        };

        match auth_actions::check_login_qr_blocking(&key, Some(cookie.as_str())) {
            Ok(response) => {
                let code = response.body.code;
                match code {
                    800 => {
                        self.runtime.login.update(cx, |login, cx| {
                            login.qr_polling = false;
                            login.qr_status = Some("800 二维码过期".to_string());
                            cx.notify();
                        });
                    }
                    801 => {
                        self.runtime.login.update(cx, |login, cx| {
                            login.qr_status = Some("801 等待扫码".to_string());
                            cx.notify();
                        });
                    }
                    802 => {
                        self.runtime.login.update(cx, |login, cx| {
                            login.qr_status = Some("802 待确认".to_string());
                            cx.notify();
                        });
                    }
                    803 => {
                        self.runtime.login.update(cx, |login, cx| {
                            login.qr_polling = false;
                            login.qr_status = Some("803 登录成功".to_string());
                            cx.notify();
                        });
                        auth::merge_auth_cookies(&self.runtime, &response.set_cookie, cx);
                        self.refresh_login_summary(cx);
                    }
                    value => {
                        self.runtime.login.update(cx, |login, cx| {
                            login.qr_status = Some(format!("{value} 登录状态未知"));
                            cx.notify();
                        });
                    }
                }
            }
            Err(err) => {
                auth::push_shell_error(&self.runtime, format!("二维码状态轮询失败: {err}"), cx);
            }
        }
        true
    }

    fn apply_guest_session_result(
        &mut self,
        result: Result<AuthBundle, String>,
        cx: &mut Context<Self>,
    ) {
        self.guest_loading = false;
        match result {
            Ok(bundle) => {
                let changed = self.runtime.session.update(cx, |session, _| {
                    if session.auth_bundle == bundle {
                        false
                    } else {
                        session.auth_bundle = bundle.clone();
                        true
                    }
                });
                if changed {
                    auth::persist_auth_bundle(&self.runtime, cx);
                }
                if bundle_has_guest_token(&bundle) {
                    self.runtime.login.update(cx, |login, cx| {
                        login.qr_status = Some("已获取游客凭据".to_string());
                        cx.notify();
                    });
                } else {
                    auth::push_shell_error(
                        &self.runtime,
                        "游客登录返回成功但未拿到 MUSIC_A".to_string(),
                        cx,
                    );
                }
            }
            Err(err) => {
                auth::push_shell_error(&self.runtime, format!("游客登录失败: {err}"), cx);
            }
        }
    }

    fn apply_login_summary_result(
        &mut self,
        expected_music_u: Option<String>,
        result: Result<LoginSummaryPayload, String>,
        cx: &mut Context<Self>,
    ) {
        self.summary_loading = false;
        if self.runtime.session.read(cx).auth_bundle.music_u != expected_music_u {
            return;
        }

        match result {
            Ok(payload) => {
                let previous_user_id = self.runtime.session.read(cx).auth_user_id;
                self.runtime.session.update(cx, |session, cx| {
                    session.auth_account_summary = payload.auth_account_summary;
                    session.auth_user_name = payload.auth_user_name;
                    session.auth_user_avatar = payload.auth_user_avatar;
                    session.auth_user_id = payload.auth_user_id;
                    cx.notify();
                });
                let next_user_id = self.runtime.session.read(cx).auth_user_id;
                if session_identity_changed(previous_user_id, next_user_id) {
                    pages::reset_session_bound_pages(&self.runtime, cx);
                }
                cx.emit(SessionEvent::SummaryRefreshed);
            }
            Err(err) => {
                self.clear_login_summary(cx);
                auth::push_shell_error(&self.runtime, format!("读取登录状态失败: {err}"), cx);
            }
        }
    }

    fn clear_login_summary(&mut self, cx: &mut Context<Self>) {
        let previous_user_id = self.runtime.session.read(cx).auth_user_id;
        self.runtime.session.update(cx, |session, cx| {
            session.auth_account_summary = None;
            session.auth_user_name = None;
            session.auth_user_avatar = None;
            session.auth_user_id = None;
            cx.notify();
        });
        if previous_user_id.is_some() {
            pages::reset_session_bound_pages(&self.runtime, cx);
        }
        cx.emit(SessionEvent::SummaryRefreshed);
    }
}

#[derive(Debug)]
struct LoginSummaryPayload {
    auth_account_summary: Option<String>,
    auth_user_name: Option<String>,
    auth_user_avatar: Option<String>,
    auth_user_id: Option<i64>,
}

fn bundle_has_user_token(bundle: &AuthBundle) -> bool {
    bundle
        .music_u
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
}

fn bundle_has_guest_token(bundle: &AuthBundle) -> bool {
    bundle_has_user_token(bundle)
        || bundle
            .music_a
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn session_identity_changed(previous_user_id: Option<i64>, next_user_id: Option<i64>) -> bool {
    previous_user_id != next_user_id
}

fn fetch_guest_bundle(mut bundle: AuthBundle) -> Result<AuthBundle, String> {
    let current_cookie = auth_actions::build_cookie_header(&bundle);
    let response = auth_actions::register_anonymous_blocking(current_cookie.as_deref())
        .map_err(|err| err.to_string())?;
    auth_actions::merge_bundle_from_set_cookie(&mut bundle, &response.set_cookie);
    if bundle_has_guest_token(&bundle) {
        Ok(bundle)
    } else {
        Err("游客登录返回成功但未拿到 MUSIC_A".to_string())
    }
}

fn fetch_login_summary(bundle: AuthBundle) -> Result<LoginSummaryPayload, String> {
    let Some(cookie) = auth_actions::build_cookie_header(&bundle) else {
        return Err("鉴权凭据异常，已阻止请求".to_string());
    };
    let body = auth_actions::fetch_login_status_blocking(Some(cookie.as_str()))
        .map_err(|err| err.to_string())?;
    let profile = auth_actions::login_profile(&body);
    Ok(LoginSummaryPayload {
        auth_account_summary: auth_actions::login_summary_text(&body),
        auth_user_name: profile.and_then(|value| value.nickname.clone()),
        auth_user_avatar: profile.and_then(|value| value.avatar_url.clone()),
        auth_user_id: body.user_id(),
    })
}
