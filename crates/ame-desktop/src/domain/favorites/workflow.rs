use nekowg::Context;
use tracing::warn;

use crate::app::runtime::AppRuntime;
use crate::domain::cache::CacheLookup;
use crate::domain::session as auth;

use super::service::{
    fetch_favorites_snapshot, invalidate_related_caches, read_favorites_cache,
    set_track_like_blocking, store_favorites_cache,
};

pub fn sync_session<T: 'static>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let user_id = runtime.session.read(cx).auth_user_id;
    let state_user_id = runtime.favorites.read(cx).user_id;

    if user_id.is_none() {
        if state_user_id.is_some() || runtime.favorites.read(cx).fetched_at_ms.is_some() {
            runtime.favorites.update(cx, |state, cx| {
                state.clear();
                cx.notify();
            });
        }
        return;
    }

    if state_user_id != user_id {
        runtime.favorites.update(cx, |state, cx| {
            state.clear();
            cx.notify();
        });
    }

    ensure_loaded(runtime, cx);
}

pub fn ensure_loaded<T: 'static>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let session = runtime.session.read(cx).clone();
    let Some(user_id) = session.auth_user_id else {
        return;
    };
    let Some(cookie) = session
        .auth_bundle
        .music_u
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .and_then(|_| auth::build_cookie_header(&session.auth_bundle))
    else {
        return;
    };

    let state = runtime.favorites.read(cx).clone();
    if state.is_ready_for(Some(user_id)) || (state.loading && state.user_id == Some(user_id)) {
        return;
    }

    let mut used_stale_cache = false;
    match read_favorites_cache(runtime, user_id) {
        Ok(CacheLookup::Fresh(cached)) => {
            runtime.favorites.update(cx, |state, cx| {
                state.apply_snapshot(cached.value, Some(cached.fetched_at_ms));
                cx.notify();
            });
            return;
        }
        Ok(CacheLookup::Stale(cached)) => {
            runtime.favorites.update(cx, |state, cx| {
                state.apply_snapshot(cached.value, Some(cached.fetched_at_ms));
                state.loading = true;
                cx.notify();
            });
            used_stale_cache = true;
        }
        Ok(CacheLookup::Miss) => {}
        Err(err) => {
            warn!(error = %err, user_id, "favorites cache read failed");
        }
    }

    runtime.favorites.update(cx, |state, cx| {
        if !used_stale_cache {
            state.begin_loading(user_id);
        } else {
            state.loading = true;
            state.error = None;
        }
        cx.notify();
    });

    let runtime = runtime.clone();
    cx.spawn(async move |_, cx| {
        let result = cx
            .background_executor()
            .spawn(async move { fetch_favorites_snapshot(user_id, &cookie) })
            .await;

        if auth::auth_user_id(&runtime, cx) != Some(user_id) {
            runtime.favorites.update(cx, |state, _| {
                if state.user_id == Some(user_id) {
                    state.loading = false;
                }
            });
            return;
        }

        runtime.favorites.update(cx, |state, cx| {
            match result {
                Ok(snapshot) => {
                    let fetched_at_ms = store_favorites_cache(&runtime, user_id, &snapshot)
                        .unwrap_or(snapshot.fetched_at_ms);
                    state.apply_snapshot(snapshot, Some(fetched_at_ms));
                }
                Err(err) => {
                    state.fail_preserving_cached(err);
                }
            }
            cx.notify();
        });
    })
    .detach();
}

pub fn toggle_track_like<T: 'static>(runtime: &AppRuntime, track_id: i64, cx: &mut Context<T>) {
    let session = runtime.session.read(cx).clone();
    let Some(user_id) = session.auth_user_id else {
        auth::push_shell_error(runtime, "收藏功能需要账号登录（MUSIC_U）".to_string(), cx);
        return;
    };
    let Some(cookie) = session
        .auth_bundle
        .music_u
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .and_then(|_| auth::build_cookie_header(&session.auth_bundle))
    else {
        auth::push_shell_error(runtime, "收藏功能需要账号登录（MUSIC_U）".to_string(), cx);
        return;
    };

    let state = runtime.favorites.read(cx).clone();
    if !state.is_ready_for(Some(user_id)) {
        ensure_loaded(runtime, cx);
        auth::push_shell_error(runtime, "收藏状态加载中，请稍后重试".to_string(), cx);
        return;
    }
    if state.is_pending(track_id) {
        return;
    }

    let liked = !state.is_liked(track_id);
    runtime.favorites.update(cx, |state, cx| {
        if state.user_id == Some(user_id) {
            state.set_pending(track_id, liked);
            cx.notify();
        }
    });

    let runtime = runtime.clone();
    cx.spawn(async move |_, cx| {
        let result = cx
            .background_executor()
            .spawn(async move { set_track_like_blocking(track_id, liked, &cookie) })
            .await;

        if auth::auth_user_id(&runtime, cx) != Some(user_id) {
            runtime.favorites.update(cx, |state, _| {
                if state.user_id == Some(user_id) {
                    state.clear_pending(track_id);
                }
            });
            return;
        }

        match result {
            Ok(fetched_at_ms) => {
                let snapshot = runtime.favorites.update(cx, |state, cx| {
                    if state.user_id != Some(user_id) {
                        return None;
                    }
                    state.apply_toggle_success(track_id, liked, fetched_at_ms);
                    cx.notify();
                    state.snapshot()
                });

                if let Some(snapshot) = snapshot {
                    if let Err(err) = store_favorites_cache(&runtime, user_id, &snapshot) {
                        warn!(error = %err, user_id, "favorites cache write failed");
                    }
                    if let Err(err) =
                        invalidate_related_caches(&runtime, user_id, snapshot.liked_playlist_id)
                    {
                        warn!(error = %err, user_id, "favorites related cache invalidation failed");
                    }
                }
            }
            Err(err) => {
                runtime.favorites.update(cx, |state, cx| {
                    if state.user_id == Some(user_id) {
                        state.fail_toggle(track_id, err.clone());
                        cx.notify();
                    }
                });
                let action = if liked { "收藏" } else { "取消收藏" };
                auth::push_shell_error(&runtime, format!("{action}失败: {err}"), cx);
            }
        }
    })
    .detach();
}
