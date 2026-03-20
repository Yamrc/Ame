use std::time::{Duration, Instant};

use nekowg::Context;

use crate::app::runtime::{
    AppRuntime, KEY_PLAYER_CURRENT_INDEX, KEY_PLAYER_DURATION_MS, KEY_PLAYER_MODE,
    KEY_PLAYER_POSITION_MS, KEY_PLAYER_QUEUE, KEY_PLAYER_VOLUME, KEY_PLAYER_WAS_PLAYING,
    KEY_WINDOW_CLOSE_BEHAVIOR, PersistedQueueItem,
};
use crate::domain::session as auth;

const PROGRESS_PERSIST_INTERVAL: Duration = Duration::from_secs(2);

pub fn persist_progress_by_interval<T>(
    runtime: &AppRuntime,
    last_persist_at: &mut Instant,
    now: Instant,
    cx: &mut Context<T>,
) {
    if now.duration_since(*last_persist_at) < PROGRESS_PERSIST_INTERVAL {
        return;
    }
    *last_persist_at = now;
    persist_player_progress(runtime, cx);
}

pub(super) fn persist_player_settings<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let Some(settings) = runtime.services.settings_store.as_ref() else {
        return;
    };
    let player = runtime.player.read(cx).clone();
    let close_behavior = runtime.shell.read(cx).close_behavior;
    let mut errors = Vec::new();
    if let Err(err) = settings.set(KEY_PLAYER_VOLUME, &player.volume) {
        errors.push(format!("保存音量失败: {err}"));
    }
    if let Err(err) = settings.set(KEY_PLAYER_MODE, &player.mode) {
        errors.push(format!("保存播放模式失败: {err}"));
    }
    if let Err(err) = settings.set(KEY_WINDOW_CLOSE_BEHAVIOR, &close_behavior) {
        errors.push(format!("保存关闭行为失败: {err}"));
    }
    for err in errors {
        auth::push_shell_error(runtime, err, cx);
    }
}

pub(super) fn persist_player_runtime<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let Some(state) = runtime.services.state_store.as_ref() else {
        return;
    };
    let player = runtime.player.read(cx).clone();
    let mut errors = Vec::new();
    let queue = player
        .queue
        .iter()
        .map(|item| PersistedQueueItem {
            id: item.id,
            name: item.name.clone(),
            alias: item.alias.clone(),
            artist: item.artist.clone(),
            album: item.album.clone(),
            duration_ms: item.duration_ms,
            cover_url: item.cover_url.clone(),
        })
        .collect::<Vec<_>>();

    if let Err(err) = state.set(KEY_PLAYER_QUEUE, &queue) {
        errors.push(format!("保存队列失败: {err}"));
    }
    if let Err(err) = state.set(KEY_PLAYER_CURRENT_INDEX, &player.current_index) {
        errors.push(format!("保存当前索引失败: {err}"));
    }
    if let Err(err) = state.set(KEY_PLAYER_WAS_PLAYING, &player.is_playing) {
        errors.push(format!("保存播放状态失败: {err}"));
    }
    for err in errors {
        auth::push_shell_error(runtime, err, cx);
    }
}

pub(super) fn persist_player_progress<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let Some(state) = runtime.services.state_store.as_ref() else {
        return;
    };
    let player = runtime.player.read(cx).clone();
    let mut errors = Vec::new();
    if let Err(err) = state.set(KEY_PLAYER_POSITION_MS, &player.position_ms) {
        errors.push(format!("保存播放进度失败: {err}"));
    }
    if let Err(err) = state.set(KEY_PLAYER_DURATION_MS, &player.duration_ms) {
        errors.push(format!("保存播放时长失败: {err}"));
    }
    for err in errors {
        auth::push_shell_error(runtime, err, cx);
    }
}
