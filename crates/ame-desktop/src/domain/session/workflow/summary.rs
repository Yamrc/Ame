use ame_core::credential::AuthBundle;
use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::session as auth;

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

fn fetch_login_summary(bundle: AuthBundle) -> Result<LoginSummaryPayload, String> {
    let Some(cookie) = auth::build_cookie_header(&bundle) else {
        return Err("鉴权凭据异常，已阻止请求".to_string());
    };
    let body =
        auth::fetch_login_status_blocking(Some(cookie.as_str())).map_err(|err| err.to_string())?;
    let profile = auth::login_profile(&body);
    Ok(LoginSummaryPayload {
        auth_account_summary: auth::login_summary_text(&body),
        auth_user_name: profile.and_then(|value| value.nickname.clone()),
        auth_user_avatar: profile.and_then(|value| value.avatar_url.clone()),
        auth_user_id: body.user_id(),
    })
}

pub fn refresh_login_summary<T: 'static>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let bundle = runtime.session.read(cx).auth_bundle.clone();
    if !bundle_has_user_token(&bundle) {
        clear_login_summary(runtime, cx);
        return;
    }
    if runtime.session.read(cx).summary_loading {
        return;
    }

    runtime.session.update(cx, |session, _| {
        session.summary_loading = true;
    });

    let expected_music_u = bundle.music_u.clone();
    let runtime = runtime.clone();
    cx.spawn(async move |_, cx| {
        let result = cx
            .background_executor()
            .spawn(async move { fetch_login_summary(bundle) })
            .await;

        let still_current = runtime.session.update(cx, |session, cx| {
            session.summary_loading = false;
            if session.auth_bundle.music_u != expected_music_u {
                return false;
            }
            match result {
                Ok(payload) => {
                    session.auth_account_summary = payload.auth_account_summary;
                    session.auth_user_name = payload.auth_user_name;
                    session.auth_user_avatar = payload.auth_user_avatar;
                    session.auth_user_id = payload.auth_user_id;
                }
                Err(err) => {
                    session.auth_account_summary = None;
                    session.auth_user_name = None;
                    session.auth_user_avatar = None;
                    session.auth_user_id = None;
                    auth::push_shell_error(&runtime, format!("读取登录状态失败: {err}"), cx);
                }
            }
            cx.notify();
            true
        });

        if !still_current {
            runtime.session.update(cx, |session, _| {
                session.summary_loading = false;
            });
        }
    })
    .detach();
}

fn clear_login_summary<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    runtime.session.update(cx, |session, cx| {
        session.auth_account_summary = None;
        session.auth_user_name = None;
        session.auth_user_avatar = None;
        session.auth_user_id = None;
        cx.notify();
    });
}
