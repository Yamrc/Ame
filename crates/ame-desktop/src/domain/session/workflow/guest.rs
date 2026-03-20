use ame_core::credential::AuthBundle;
use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::session as auth;

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

fn fetch_guest_bundle(mut bundle: AuthBundle) -> Result<AuthBundle, String> {
    let current_cookie = auth::build_cookie_header(&bundle);
    let response = auth::register_anonymous_blocking(current_cookie.as_deref())
        .map_err(|err| err.to_string())?;
    auth::merge_bundle_from_set_cookie(&mut bundle, &response.set_cookie);
    if bundle_has_guest_token(&bundle) {
        Ok(bundle)
    } else {
        Err("游客登录返回成功但未拿到 MUSIC_A".to_string())
    }
}

pub fn ensure_guest_session<T: 'static>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let bundle = runtime.session.read(cx).auth_bundle.clone();
    let loading = runtime.session.read(cx).guest_loading;
    if loading || bundle_has_guest_token(&bundle) {
        return;
    }

    runtime.session.update(cx, |session, _| {
        session.guest_loading = true;
    });

    let runtime = runtime.clone();
    cx.spawn(async move |_, cx| {
        let result = cx
            .background_executor()
            .spawn(async move { fetch_guest_bundle(bundle) })
            .await;

        match result {
            Ok(bundle) => {
                let changed = runtime.session.update(cx, |session, _| {
                    session.guest_loading = false;
                    if session.auth_bundle == bundle {
                        false
                    } else {
                        session.auth_bundle = bundle.clone();
                        true
                    }
                });
                if changed {
                    auth::persist_auth_bundle(&runtime, cx);
                }
                if !bundle_has_guest_token(&bundle) {
                    auth::push_shell_error(
                        &runtime,
                        "游客登录返回成功但未拿到 MUSIC_A".to_string(),
                        cx,
                    );
                }
            }
            Err(err) => {
                runtime.session.update(cx, |session, _| {
                    session.guest_loading = false;
                });
                auth::push_shell_error(&runtime, format!("游客登录失败: {err}"), cx);
            }
        }
    })
    .detach();
}
