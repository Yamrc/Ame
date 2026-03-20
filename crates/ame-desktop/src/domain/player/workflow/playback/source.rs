use ame_audio::{AudioCommand, SourceSpec};
use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::player;
use crate::domain::session as auth;
use crate::domain::session::AuthLevel;

use super::super::bridge::with_audio_bridge_or_error;
use super::super::persist::persist_player_runtime;

fn prepare_track_source<T>(
    runtime: &AppRuntime,
    track_id: i64,
    queue_index: usize,
    cx: &mut Context<T>,
) -> Option<String> {
    let cookie = auth::ensure_auth_cookie(runtime, AuthLevel::Guest, cx)?;
    let source_url = match player::fetch_track_url_blocking(track_id, Some(cookie.as_str())) {
        Ok(url) => url,
        Err(err) => {
            auth::set_shell_error(runtime, Some(format!("获取播放地址失败: {err}")), cx);
            return None;
        }
    };

    runtime.player.update(cx, |player, _| {
        if let Some(item) = player.queue.get_mut(queue_index) {
            item.source_url = Some(source_url.clone());
        }
    });
    Some(source_url)
}

pub(in crate::domain::player::workflow) fn start_playback_at<T>(
    runtime: &AppRuntime,
    queue_index: usize,
    start_ms: u64,
    autoplay: bool,
    cx: &mut Context<T>,
) -> bool {
    let snapshot = runtime.player.read(cx).clone();
    let Some(item) = snapshot.queue.get(queue_index).cloned() else {
        return false;
    };

    let Some(source_url) = prepare_track_source(runtime, item.id, queue_index, cx) else {
        persist_player_runtime(runtime, cx);
        return false;
    };

    let opened = with_audio_bridge_or_error(runtime, cx, "播放失败", |audio| {
        audio.send(AudioCommand::Open {
            source: SourceSpec::network(source_url),
            start_ms,
            autoplay,
        })
    });
    match opened {
        Some(Ok(_)) => {}
        Some(Err(err)) => {
            auth::set_shell_error(runtime, Some(format!("播放失败: {err}")), cx);
            return false;
        }
        None => {
            return false;
        }
    }

    runtime.player.update(cx, |player, cx| {
        player.current_index = Some(queue_index);
        player.is_playing = autoplay;
        cx.notify();
    });
    persist_player_runtime(runtime, cx);
    true
}

pub(in crate::domain::player::workflow) fn refresh_current_track_url_and_resume<T>(
    runtime: &AppRuntime,
    cx: &mut Context<T>,
) -> bool {
    let player_snapshot = runtime.player.read(cx).clone();
    let Some(current_index) = player_snapshot.current_index else {
        return false;
    };
    let Some(current_item) = player_snapshot.queue.get(current_index).cloned() else {
        return false;
    };

    let Some(url) = prepare_track_source(runtime, current_item.id, current_index, cx) else {
        auth::set_shell_error(runtime, Some("刷新播放地址失败".to_string()), cx);
        return false;
    };

    let reopened = with_audio_bridge_or_error(runtime, cx, "刷新播放失败", |audio| {
        let position = audio.service().snapshot().position_ms;
        audio.send(AudioCommand::Open {
            source: SourceSpec::network(url),
            start_ms: position,
            autoplay: true,
        })
    });
    match reopened {
        Some(Ok(_)) => {}
        Some(Err(err)) => {
            auth::set_shell_error(runtime, Some(format!("刷新播放失败: {err}")), cx);
            return false;
        }
        None => {
            return false;
        }
    }

    runtime.player.update(cx, |player, cx| {
        player.is_playing = true;
        cx.notify();
    });
    persist_player_runtime(runtime, cx);
    true
}
