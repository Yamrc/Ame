use ame_core::credential::AuthBundle;
use nekowg::AppContext;

use crate::app::runtime::AppRuntime;
use crate::app::runtime::KEY_SESSION_IDENTITY;
use crate::domain::cache::{CacheClass, CacheScope};

use super::service as auth_actions;
use super::state::PersistedSessionIdentity;

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
        push_shell_error(
            runtime,
            format!("Failed to write keyring credentials: {err}"),
            cx,
        );
    }
}

pub fn persist_session_identity<C: AppContext>(runtime: &AppRuntime, cx: &mut C) {
    let Some(state_store) = runtime.services.state_store.as_ref() else {
        return;
    };
    let identity = runtime.session.read_with(cx, |session, _| {
        PersistedSessionIdentity::from_session(session)
    });
    let result = match identity {
        Some(identity) => state_store.set(KEY_SESSION_IDENTITY, &identity),
        None => state_store.remove(KEY_SESSION_IDENTITY),
    };
    if let Err(err) = result {
        push_shell_error(
            runtime,
            format!("Failed to write session identity snapshot: {err}"),
            cx,
        );
    }
}

pub fn clear_persisted_session_identity<C: AppContext>(runtime: &AppRuntime, cx: &mut C) {
    let Some(state_store) = runtime.services.state_store.as_ref() else {
        return;
    };
    if let Err(err) = state_store.remove(KEY_SESSION_IDENTITY) {
        push_shell_error(
            runtime,
            format!("Failed to remove session identity snapshot: {err}"),
            cx,
        );
    }
}

pub fn invalidate_firework_for_identity_transition<C: AppContext>(
    runtime: &AppRuntime,
    old_user_id: Option<i64>,
    new_user_id: Option<i64>,
    cx: &mut C,
) {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return;
    };
    let mut scopes = vec![CacheScope::Guest];
    if let Some(user_id) = old_user_id {
        scopes.push(CacheScope::User(user_id));
    }
    if let Some(user_id) = new_user_id
        && !scopes
            .iter()
            .any(|scope| scope == &CacheScope::User(user_id))
    {
        scopes.push(CacheScope::User(user_id));
    }

    for scope in scopes {
        if let Err(err) = cache.invalidate_scope(CacheClass::Firework, &scope) {
            push_shell_error(
                runtime,
                format!("Failed to invalidate firework cache ({scope}): {err}"),
                cx,
            );
        }
    }
}

pub fn merge_auth_cookies<C: AppContext>(
    runtime: &AppRuntime,
    set_cookie: &[String],
    cx: &mut C,
) -> bool {
    let mut bundle = auth_bundle(runtime, cx);
    let old_fingerprint = super::state::session_identity_fingerprint(&bundle);
    let old_user_id = auth_user_id(runtime, cx);
    let changed = auth_actions::merge_bundle_from_set_cookie(&mut bundle, set_cookie);
    if changed {
        runtime.session.update(cx, |session, _| {
            session.auth_bundle = bundle;
        });
        persist_auth_bundle(runtime, cx);
        let current_bundle = auth_bundle(runtime, cx);
        if super::state::session_identity_fingerprint(&current_bundle) != old_fingerprint {
            invalidate_firework_for_identity_transition(runtime, old_user_id, None, cx);
            clear_persisted_session_identity(runtime, cx);
        }
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
                push_shell_error(
                    runtime,
                    "Guest login succeeded but MUSIC_A was missing".to_string(),
                    cx,
                );
                false
            }
        }
        Err(err) => {
            push_shell_error(runtime, format!("Guest login failed: {err}"), cx);
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
                push_shell_error(
                    runtime,
                    "This action requires account login credentials (MUSIC_U)".to_string(),
                    cx,
                );
                false
            }
        }
    };
    if !ok {
        return None;
    }

    let cookie = auth_actions::build_cookie_header(&auth_bundle(runtime, cx));
    if cookie.is_none() {
        push_shell_error(
            runtime,
            "Invalid auth credentials blocked the request".to_string(),
            cx,
        );
    }
    cookie
}
