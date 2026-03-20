use ame_core::credential::AuthBundle;
use nekowg::AppContext;

use crate::app::runtime::AppRuntime;

use super::service as auth_actions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthLevel {
    Guest,
    User,
}

pub fn auth_bundle<C: AppContext>(runtime: &AppRuntime, cx: &C) -> AuthBundle {
    runtime
        .session
        .read_with(cx, |session, _| session.auth_bundle.clone())
}

pub fn auth_user_id<C: AppContext>(runtime: &AppRuntime, cx: &C) -> Option<i64> {
    runtime
        .session
        .read_with(cx, |session, _| session.auth_user_id)
}

pub fn has_user_token<C: AppContext>(runtime: &AppRuntime, cx: &C) -> bool {
    runtime.session.read_with(cx, |session, _| {
        session
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    })
}

pub fn has_guest_token<C: AppContext>(runtime: &AppRuntime, cx: &C) -> bool {
    has_user_token(runtime, cx)
        || runtime.session.read_with(cx, |session, _| {
            session
                .auth_bundle
                .music_a
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
        })
}

pub fn set_shell_error<C: AppContext>(runtime: &AppRuntime, error: Option<String>, cx: &mut C) {
    runtime.shell.update(cx, |shell, cx| {
        shell.error = error;
        cx.notify();
    });
}

pub fn push_shell_error<C: AppContext>(
    runtime: &AppRuntime,
    message: impl Into<String>,
    cx: &mut C,
) {
    let message = message.into();
    if message.trim().is_empty() {
        return;
    }

    runtime.shell.update(cx, |shell, cx| {
        match &mut shell.error {
            Some(existing) => {
                existing.push('\n');
                existing.push_str(&message);
            }
            None => shell.error = Some(message),
        }
        cx.notify();
    });
}

pub fn persist_auth_bundle<C: AppContext>(runtime: &AppRuntime, cx: &mut C) {
    let bundle = auth_bundle(runtime, cx);
    if let Err(err) = runtime.services.credential_store.save_auth_bundle(&bundle) {
        push_shell_error(runtime, format!("写入 keyring 凭据失败: {err}"), cx);
    }
}

pub fn merge_auth_cookies<C: AppContext>(
    runtime: &AppRuntime,
    set_cookie: &[String],
    cx: &mut C,
) -> bool {
    let mut bundle = auth_bundle(runtime, cx);
    let changed = auth_actions::merge_bundle_from_set_cookie(&mut bundle, set_cookie);
    if changed {
        runtime.session.update(cx, |session, _| {
            session.auth_bundle = bundle;
        });
        persist_auth_bundle(runtime, cx);
    }
    changed
}

pub fn ensure_guest_token<C: AppContext>(runtime: &AppRuntime, cx: &mut C) -> bool {
    if has_guest_token(runtime, cx) {
        return true;
    }

    let current_cookie = auth_actions::build_cookie_header(&auth_bundle(runtime, cx));
    match auth_actions::register_anonymous_blocking(current_cookie.as_deref()) {
        Ok(response) => {
            merge_auth_cookies(runtime, &response.set_cookie, cx);
            if runtime.session.read_with(cx, |session, _| {
                session
                    .auth_bundle
                    .music_a
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
            }) {
                true
            } else {
                push_shell_error(runtime, "游客登录返回成功但未拿到 MUSIC_A".to_string(), cx);
                false
            }
        }
        Err(err) => {
            push_shell_error(runtime, format!("游客登录失败: {err}"), cx);
            false
        }
    }
}

pub fn ensure_auth_cookie<C: AppContext>(
    runtime: &AppRuntime,
    level: AuthLevel,
    cx: &mut C,
) -> Option<String> {
    let ok = match level {
        AuthLevel::Guest => ensure_guest_token(runtime, cx),
        AuthLevel::User => {
            if has_user_token(runtime, cx) {
                true
            } else {
                push_shell_error(runtime, "当前操作需要账号登录凭据(MUSIC_U)".to_string(), cx);
                false
            }
        }
    };
    if !ok {
        return None;
    }

    let cookie = auth_actions::build_cookie_header(&auth_bundle(runtime, cx));
    if cookie.is_none() {
        push_shell_error(runtime, "鉴权凭据异常，已阻止请求".to_string(), cx);
    }
    cookie
}
