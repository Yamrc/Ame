use ame_core::credential::AuthBundle;
use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::session as auth;
use crate::domain::session::SessionState;

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
        return Err("Invalid auth credentials blocked the request".to_string());
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

fn apply_login_summary_payload(session: &mut SessionState, payload: LoginSummaryPayload) {
    session.auth_account_summary = payload.auth_account_summary;
    session.auth_user_name = payload.auth_user_name;
    session.auth_user_avatar = payload.auth_user_avatar;
    session.auth_user_id = payload.auth_user_id;
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
        let persist_identity = result.is_ok();
        let mut old_user_id = None;
        let mut new_user_id = None;

        let still_current = runtime.session.update(cx, |session, cx| {
            session.summary_loading = false;
            if session.auth_bundle.music_u != expected_music_u {
                return false;
            }
            old_user_id = session.auth_user_id;
            match result {
                Ok(payload) => {
                    apply_login_summary_payload(session, payload);
                    new_user_id = session.auth_user_id;
                }
                Err(err) => {
                    new_user_id = session.auth_user_id;
                    auth::push_shell_error(
                        &runtime,
                        format!("Failed to read login status: {err}"),
                        cx,
                    );
                }
            }
            cx.notify();
            true
        });
        if persist_identity && still_current {
            auth::persist_session_identity(&runtime, cx);
            if old_user_id != new_user_id {
                auth::invalidate_firework_for_identity_transition(
                    &runtime,
                    old_user_id,
                    new_user_id,
                    cx,
                );
            }
        }

        if !still_current {
            runtime.session.update(cx, |session, _| {
                session.summary_loading = false;
            });
        }
    })
    .detach();
}

fn clear_login_summary<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let old_user_id = runtime.session.read(cx).auth_user_id;
    runtime.session.update(cx, |session, cx| {
        session.auth_account_summary = None;
        session.auth_user_name = None;
        session.auth_user_avatar = None;
        session.auth_user_id = None;
        cx.notify();
    });
    auth::invalidate_firework_for_identity_transition(runtime, old_user_id, None, cx);
    auth::clear_persisted_session_identity(runtime, cx);
}

#[cfg(test)]
mod tests {
    use ame_core::credential::AuthBundle;

    use super::{LoginSummaryPayload, apply_login_summary_payload};
    use crate::domain::session::SessionState;

    #[test]
    fn applying_summary_payload_updates_identity_fields() {
        let mut session = SessionState {
            auth_bundle: AuthBundle {
                music_u: Some("token".to_string()),
                ..Default::default()
            },
            auth_account_summary: Some("old summary".to_string()),
            auth_user_name: Some("old name".to_string()),
            auth_user_avatar: Some("old avatar".to_string()),
            auth_user_id: Some(1),
            ..Default::default()
        };

        apply_login_summary_payload(
            &mut session,
            LoginSummaryPayload {
                auth_account_summary: Some("new summary".to_string()),
                auth_user_name: Some("new name".to_string()),
                auth_user_avatar: Some("new avatar".to_string()),
                auth_user_id: Some(2),
            },
        );

        assert_eq!(session.auth_account_summary.as_deref(), Some("new summary"));
        assert_eq!(session.auth_user_name.as_deref(), Some("new name"));
        assert_eq!(session.auth_user_avatar.as_deref(), Some("new avatar"));
        assert_eq!(session.auth_user_id, Some(2));
    }
}
