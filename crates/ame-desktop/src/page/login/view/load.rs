use std::sync::Arc;
use std::time::{Duration, Instant};

use nekowg::{Context, Image};

use crate::domain::session as auth;

use super::LoginPageView;

impl LoginPageView {
    pub(super) fn generate_login_qr(&mut self, cx: &mut Context<Self>) {
        let Some(cookie) = auth::ensure_auth_cookie(&self.runtime, auth::AuthLevel::Guest, cx)
        else {
            return;
        };
        let response = match auth::fetch_login_qr_key_blocking(Some(cookie.as_str())) {
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
        let image_data = match qrcode::QrCode::new(qr_url.as_bytes()) {
            Ok(code) => {
                let svg = code
                    .render::<qrcode::render::svg::Color<'_>>()
                    .min_dimensions(280, 280)
                    .build();
                Some(Arc::new(Image::from_bytes(
                    nekowg::ImageFormat::Svg,
                    svg.into_bytes(),
                )))
            }
            Err(err) => {
                auth::push_shell_error(&self.runtime, format!("渲染二维码失败: {err}"), cx);
                None
            }
        };

        self.state.update(cx, |login, cx| {
            login.qr_key = Some(key);
            login.qr_url = Some(qr_url);
            login.qr_image = image_data;
            login.qr_status = Some("801 等待扫码".to_string());
            login.qr_polling = true;
            login.qr_poll_started_at = Some(Instant::now());
            login.qr_last_polled_at = None;
            cx.notify();
        });

        self.start_qr_polling(cx);
    }

    fn start_qr_polling(&mut self, cx: &mut Context<Self>) {
        if self.polling_task_active {
            return;
        }
        if !self.state.read(cx).qr_polling {
            return;
        }

        self.polling_task_active = true;
        let page = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let updated = page.update(cx, |this, cx| {
                    let now = Instant::now();
                    let keep = this.tick_qr_poll(now, cx);
                    if !keep {
                        this.polling_task_active = false;
                    }
                    keep
                });
                match updated {
                    Ok(true) => {}
                    _ => break,
                }
            }
        })
        .detach();
    }

    pub(super) fn stop_login_qr_polling(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |login, cx| {
            login.qr_polling = false;
            login.qr_status = Some("已停止轮询".to_string());
            cx.notify();
        });
    }

    fn tick_qr_poll(&mut self, now: Instant, cx: &mut Context<Self>) -> bool {
        let login = self.state.read(cx).clone();
        if !login.qr_polling {
            return false;
        }

        if let Some(started_at) = login.qr_poll_started_at
            && now.duration_since(started_at) >= Duration::from_secs(120)
        {
            self.state.update(cx, |login, cx| {
                login.qr_polling = false;
                login.qr_status = Some("800 二维码过期".to_string());
                cx.notify();
            });
            return false;
        }

        if let Some(last) = login.qr_last_polled_at
            && now.duration_since(last) < Duration::from_secs(2)
        {
            return true;
        }

        let Some(key) = login.qr_key.clone() else {
            self.state.update(cx, |login, cx| {
                login.qr_polling = false;
                login.qr_status = Some("二维码 key 丢失".to_string());
                cx.notify();
            });
            return false;
        };

        self.state.update(cx, |login, _| {
            login.qr_last_polled_at = Some(now);
        });

        let Some(cookie) = auth::ensure_auth_cookie(&self.runtime, auth::AuthLevel::Guest, cx)
        else {
            self.state.update(cx, |login, cx| {
                login.qr_polling = false;
                cx.notify();
            });
            return false;
        };

        match auth::check_login_qr_blocking(&key, Some(cookie.as_str())) {
            Ok(response) => {
                let code = response.body.code;
                match code {
                    800 => {
                        self.state.update(cx, |login, cx| {
                            login.qr_polling = false;
                            login.qr_status = Some("800 二维码过期".to_string());
                            cx.notify();
                        });
                    }
                    801 => {
                        self.state.update(cx, |login, cx| {
                            login.qr_status = Some("801 等待扫码".to_string());
                            cx.notify();
                        });
                    }
                    802 => {
                        self.state.update(cx, |login, cx| {
                            login.qr_status = Some("802 待确认".to_string());
                            cx.notify();
                        });
                    }
                    803 => {
                        self.state.update(cx, |login, cx| {
                            login.qr_polling = false;
                            login.qr_status = Some("803 登录成功".to_string());
                            cx.notify();
                        });
                        auth::merge_auth_cookies(&self.runtime, &response.set_cookie, cx);
                        auth::refresh_login_summary(&self.runtime, cx);
                    }
                    value => {
                        self.state.update(cx, |login, cx| {
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

        self.state.read(cx).qr_polling
    }
}
