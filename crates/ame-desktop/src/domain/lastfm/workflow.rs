use nekowg::{AppContext, Context};

use crate::app::runtime::AppRuntime;
use crate::domain::lastfm::service::{
    self, LastFmNowPlayingPayload, fetch_user_name, retry_backoff_ms, scrobble,
    store_scrobble_queue, update_now_playing,
};
use crate::domain::lastfm::{LastFmBuildConfig, LastFmCurrentPlayback, LastFmError, LastFmState};
use crate::domain::player::QueueItem;
use crate::domain::session as auth;

pub fn prime_session<T: 'static>(runtime: &AppRuntime, cx: &mut Context<T>) {
    if !runtime.lastfm.read(cx).configured {
        return;
    }
    let Some(session_key) = runtime
        .lastfm
        .read(cx)
        .session
        .as_ref()
        .map(|session| session.session_key.clone())
    else {
        return;
    };
    if runtime.lastfm.read(cx).auth_inflight {
        return;
    }

    runtime.lastfm.update(cx, |state, _| {
        state.auth_inflight = true;
        state.error = None;
    });

    let runtime = runtime.clone();
    cx.spawn(async move |_, cx| {
        let config = LastFmBuildConfig::from_env();
        let lookup_session_key = session_key.clone();
        let result = cx
            .background_executor()
            .spawn(async move { fetch_user_name(config, &lookup_session_key) })
            .await;

        match result {
            Ok(user_name) => {
                runtime.lastfm.update(cx, |state, cx| {
                    state.auth_inflight = false;
                    if let Some(session) = state.session.as_mut()
                        && session.session_key == session_key
                    {
                        session.user_name = Some(user_name);
                    }
                    state.error = None;
                    cx.notify();
                });
            }
            Err(LastFmError::InvalidSession(message)) => {
                handle_invalid_session(
                    &runtime,
                    Some(format!("Last.fm session expired: {message}")),
                    cx,
                );
            }
            Err(err) => {
                runtime.lastfm.update(cx, |state, cx| {
                    state.auth_inflight = false;
                    state.error = Some(err.to_string());
                    cx.notify();
                });
            }
        }
    })
    .detach();
}

pub fn connect<T: 'static>(runtime: &AppRuntime, cx: &mut Context<T>) {
    if !runtime.lastfm.read(cx).configured {
        let message = "Last.fm is not configured in this build".to_string();
        runtime.lastfm.update(cx, |state, cx| {
            state.error = Some(message.clone());
            cx.notify();
        });
        auth::push_shell_error(runtime, message, cx);
        return;
    }
    if runtime.lastfm.read(cx).auth_inflight {
        return;
    }

    runtime.lastfm.update(cx, |state, cx| {
        state.auth_inflight = true;
        state.error = None;
        cx.notify();
    });

    let runtime = runtime.clone();
    cx.spawn(async move |_, cx| {
        let config = LastFmBuildConfig::from_env();
        let result = cx
            .background_executor()
            .spawn(async move { service::authenticate_via_browser(config) })
            .await;

        match result {
            Ok(session) => {
                if let Err(err) = runtime
                    .services
                    .credential_store
                    .save_lastfm_session_key(&session.session_key)
                {
                    let message = format!("Failed to write Last.fm session key: {err}");
                    runtime.lastfm.update(cx, |state, cx| {
                        state.auth_inflight = false;
                        state.error = Some(message.clone());
                        cx.notify();
                    });
                    auth::push_shell_error(&runtime, message, cx);
                    return;
                }

                runtime.lastfm.update(cx, |state, cx| {
                    state.session = Some(session);
                    state.auth_inflight = false;
                    state.error = None;
                    cx.notify();
                });
            }
            Err(err) => {
                runtime.lastfm.update(cx, |state, cx| {
                    state.auth_inflight = false;
                    state.error = Some(err.to_string());
                    cx.notify();
                });
                auth::push_shell_error(&runtime, format!("Last.fm connect failed: {err}"), cx);
            }
        }
    })
    .detach();
}

pub fn disconnect<T: 'static>(runtime: &AppRuntime, cx: &mut Context<T>) {
    if let Err(err) = runtime
        .services
        .credential_store
        .delete_lastfm_session_key()
    {
        auth::push_shell_error(
            runtime,
            format!("Failed to delete Last.fm session key: {err}"),
            cx,
        );
    }

    runtime.lastfm.update(cx, |state, cx| {
        state.clear_session();
        cx.notify();
    });
    persist_scrobble_queue(runtime, cx);
}

pub fn handle_playback_started<T: 'static>(
    runtime: &AppRuntime,
    item: &QueueItem,
    cx: &mut Context<T>,
) {
    if !runtime.lastfm.read(cx).configured || !runtime.lastfm.read(cx).is_connected() {
        runtime.lastfm.update(cx, |state, _| {
            state.current_playback = None;
        });
        return;
    }

    let now_ms = service::now_millis();
    let now_unix_secs = service::now_unix_secs();
    let Some(candidate) = LastFmCurrentPlayback::from_queue_item(item, now_ms, now_unix_secs)
    else {
        runtime.lastfm.update(cx, |state, _| {
            state.current_playback = None;
        });
        return;
    };

    let queue_changed = runtime.lastfm.update(cx, |state, cx| {
        let changed = finalize_current_playback_locked(state, now_ms);
        state.current_playback = Some(candidate.clone());
        cx.notify();
        changed
    });
    if queue_changed {
        persist_scrobble_queue(runtime, cx);
    }

    spawn_now_playing(runtime, item, candidate.started_at_unix_secs, cx);
}

pub fn finalize_playback<T: 'static>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let now_ms = service::now_millis();
    let queue_changed = runtime.lastfm.update(cx, |state, cx| {
        let changed = finalize_current_playback_locked(state, now_ms);
        state.current_playback = None;
        cx.notify();
        changed
    });
    if queue_changed {
        persist_scrobble_queue(runtime, cx);
    }
}

pub fn tick<T: 'static>(runtime: &AppRuntime, now_ms: u64, cx: &mut Context<T>) {
    let player = runtime.player.read(cx).clone();

    let queue_changed = runtime.lastfm.update(cx, |state, cx| {
        let Some(track_id) = state
            .current_playback
            .as_ref()
            .map(|candidate| candidate.track_id)
        else {
            return false;
        };

        if player.current_item().map(|item| item.id) != Some(track_id) {
            let changed = finalize_current_playback_locked(state, now_ms);
            state.current_playback = None;
            cx.notify();
            return changed;
        }

        let record = {
            let candidate = state
                .current_playback
                .as_mut()
                .expect("candidate should still exist");
            candidate.refresh_duration(player.duration_ms);
            if player.is_playing {
                if let Some(last_tick_at_ms) = candidate.last_tick_at_ms {
                    candidate.played_ms = candidate
                        .played_ms
                        .saturating_add(now_ms.saturating_sub(last_tick_at_ms));
                }
                candidate.last_tick_at_ms = Some(now_ms);
            } else {
                candidate.last_tick_at_ms = None;
            }

            if !candidate.scrobbled && candidate.should_scrobble() {
                candidate.scrobbled = true;
                Some(candidate.to_scrobble_record(now_ms))
            } else {
                None
            }
        };

        if let Some(record) = record {
            let changed = state.enqueue_scrobble(record);
            if changed {
                cx.notify();
            }
            return changed;
        }

        false
    });
    if queue_changed {
        persist_scrobble_queue(runtime, cx);
    }

    flush_queue_if_due(runtime, now_ms, cx);
}

fn flush_queue_if_due<T: 'static>(runtime: &AppRuntime, now_ms: u64, cx: &mut Context<T>) {
    let state = runtime.lastfm.read(cx).clone();
    if !state.configured
        || state.auth_inflight
        || state.queue_flush_inflight
        || !state.is_connected()
    {
        return;
    }

    let Some(record) = state.next_due_scrobble(now_ms) else {
        return;
    };
    let Some(session_key) = state
        .session
        .as_ref()
        .map(|session| session.session_key.clone())
    else {
        return;
    };

    runtime.lastfm.update(cx, |state, _| {
        state.queue_flush_inflight = true;
    });

    let runtime = runtime.clone();
    cx.spawn(async move |_, cx| {
        let config = LastFmBuildConfig::from_env();
        let queued_record = record.clone();
        let result = cx
            .background_executor()
            .spawn(async move { scrobble(config, &session_key, &queued_record) })
            .await;

        match result {
            Ok(()) => {
                let changed = runtime.lastfm.update(cx, |state, cx| {
                    state.queue_flush_inflight = false;
                    state.error = None;
                    let changed = state.remove_scrobble(&record);
                    if changed {
                        cx.notify();
                    }
                    changed
                });
                if changed {
                    persist_scrobble_queue(&runtime, cx);
                }
            }
            Err(LastFmError::InvalidSession(message)) => {
                handle_invalid_session(
                    &runtime,
                    Some(format!(
                        "Last.fm session expired while scrobbling: {message}"
                    )),
                    cx,
                );
            }
            Err(err) if err.is_retryable() => {
                let retry_count = record.retry_count.saturating_add(1);
                let next_retry_at_ms =
                    service::now_millis().saturating_add(retry_backoff_ms(retry_count));
                let changed = runtime.lastfm.update(cx, |state, cx| {
                    state.queue_flush_inflight = false;
                    state.error = Some(err.to_string());
                    let changed =
                        state.update_scrobble_retry(&record, retry_count, next_retry_at_ms);
                    if changed {
                        cx.notify();
                    }
                    changed
                });
                if changed {
                    persist_scrobble_queue(&runtime, cx);
                }
            }
            Err(err) => {
                let changed = runtime.lastfm.update(cx, |state, cx| {
                    state.queue_flush_inflight = false;
                    state.error = Some(err.to_string());
                    let changed = state.remove_scrobble(&record);
                    if changed {
                        cx.notify();
                    }
                    changed
                });
                if changed {
                    persist_scrobble_queue(&runtime, cx);
                }
                auth::push_shell_error(&runtime, format!("Last.fm scrobble failed: {err}"), cx);
            }
        }
    })
    .detach();
}

fn spawn_now_playing<T: 'static>(
    runtime: &AppRuntime,
    item: &QueueItem,
    started_at_unix_secs: u64,
    cx: &mut Context<T>,
) {
    let Some(session_key) = runtime
        .lastfm
        .read(cx)
        .session
        .as_ref()
        .map(|session| session.session_key.clone())
    else {
        return;
    };
    let payload = LastFmNowPlayingPayload {
        artist: item.artist.clone(),
        track: item.name.clone(),
        album: item.album.clone(),
        duration_ms: item.duration_ms,
    };
    let runtime = runtime.clone();
    cx.spawn(async move |_, cx| {
        let config = LastFmBuildConfig::from_env();
        let queued_payload = payload.clone();
        let result = cx
            .background_executor()
            .spawn(async move { update_now_playing(config, &session_key, &queued_payload) })
            .await;

        match result {
            Ok(()) => {
                runtime.lastfm.update(cx, |state, cx| {
                    if let Some(candidate) = state.current_playback.as_mut()
                        && candidate.started_at_unix_secs == started_at_unix_secs
                        && candidate.track == payload.track
                        && candidate.artist == payload.artist
                    {
                        candidate.now_playing_sent = true;
                    }
                    state.error = None;
                    cx.notify();
                });
            }
            Err(LastFmError::InvalidSession(message)) => {
                handle_invalid_session(
                    &runtime,
                    Some(format!(
                        "Last.fm session expired while updating now playing: {message}"
                    )),
                    cx,
                );
            }
            Err(err) => {
                runtime.lastfm.update(cx, |state, cx| {
                    state.error = Some(err.to_string());
                    cx.notify();
                });
            }
        }
    })
    .detach();
}

fn finalize_current_playback_locked(state: &mut LastFmState, now_ms: u64) -> bool {
    let record = {
        let Some(candidate) = state.current_playback.as_mut() else {
            return false;
        };
        if !candidate.scrobbled && candidate.should_scrobble() {
            candidate.scrobbled = true;
            Some(candidate.to_scrobble_record(now_ms))
        } else {
            None
        }
    };
    let Some(record) = record else {
        return false;
    };
    state.enqueue_scrobble(record)
}

fn handle_invalid_session<C: AppContext>(
    runtime: &AppRuntime,
    message: Option<String>,
    cx: &mut C,
) {
    if let Err(err) = runtime
        .services
        .credential_store
        .delete_lastfm_session_key()
    {
        auth::push_shell_error(
            runtime,
            format!("Failed to delete invalid Last.fm session key: {err}"),
            cx,
        );
    }

    let shell_message =
        message.unwrap_or_else(|| "Last.fm session is invalid. Please reconnect.".to_string());
    runtime.lastfm.update(cx, |state, cx| {
        state.clear_session();
        state.error = Some(shell_message.clone());
        cx.notify();
    });
    persist_scrobble_queue(runtime, cx);
    auth::push_shell_error(runtime, shell_message, cx);
}

fn persist_scrobble_queue<C: AppContext>(runtime: &AppRuntime, cx: &mut C) {
    let Some(state_store) = runtime.services.state_store.as_ref() else {
        return;
    };
    let queue = runtime
        .lastfm
        .read_with(cx, |state, _| state.scrobble_queue.clone());
    if let Err(err) = store_scrobble_queue(state_store, &queue) {
        auth::push_shell_error(runtime, err, cx);
    }
}
