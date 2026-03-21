use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::session as auth;

use super::summary::refresh_login_summary;

pub fn refresh_login_token<T: 'static>(runtime: &AppRuntime, cx: &mut Context<T>) {
    if !auth::has_user_token(runtime, cx) {
        auth::push_shell_error(
            runtime,
            "Current session is not an account login; cannot refresh login token".to_string(),
            cx,
        );
        return;
    }

    let Some(cookie) = auth::ensure_auth_cookie(runtime, auth::AuthLevel::User, cx) else {
        return;
    };

    match auth::refresh_login_token_blocking(Some(cookie.as_str())) {
        Ok(response) => {
            auth::merge_auth_cookies(runtime, &response.set_cookie, cx);
            refresh_login_summary(runtime, cx);
        }
        Err(err) => {
            auth::push_shell_error(runtime, format!("Failed to refresh login token: {err}"), cx);
        }
    }
}
