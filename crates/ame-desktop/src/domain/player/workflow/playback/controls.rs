use ame_audio::{AudioCommand, AudioError, SeekTarget};
use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::session as auth;

use super::super::bridge::{
    set_shell_error_if_changed, with_audio_bridge, with_audio_bridge_or_error,
};
use super::super::persist::{
    persist_player_progress, persist_player_runtime, persist_player_settings,
};
use super::{refresh_current_track_url_and_resume, start_playback_at};

pub fn set_volume_absolute<T>(runtime: &AppRuntime, volume: f32, cx: &mut Context<T>) {
    let volume = volume.clamp(0.0, 1.0);
    runtime.player.update(cx, |player, cx| {
        player.set_volume(volume);
        cx.notify();
    });
    match with_audio_bridge(runtime, |audio| audio.send(AudioCommand::SetVolume(volume))) {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => auth::push_shell_error(runtime, format!("设置音量失败: {err}"), cx),
        Err(err) => auth::push_shell_error(runtime, format!("设置音量失败: {err}"), cx),
    }
    persist_player_settings(runtime, cx);
}

pub fn preview_seek_ratio<T>(runtime: &AppRuntime, ratio: f32, cx: &mut Context<T>) {
    let ratio = ratio.clamp(0.0, 1.0);
    runtime.player.update(cx, |player, cx| {
        let duration = player.duration_ms.max(1);
        player.position_ms = ((duration as f32) * ratio) as u64;
        cx.notify();
    });
}

pub fn commit_seek_ratio<T>(runtime: &AppRuntime, ratio: f32, cx: &mut Context<T>) {
    let ratio = ratio.clamp(0.0, 1.0);
    let duration_ms = runtime.player.read(cx).duration_ms.max(1);
    let target_ms = ((duration_ms as f32) * ratio) as u64;
    match with_audio_bridge(runtime, |audio| {
        audio.send(AudioCommand::Seek(SeekTarget::ms(target_ms)))
    }) {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => auth::push_shell_error(runtime, format!("拖动进度失败: {err}"), cx),
        Err(err) => auth::push_shell_error(runtime, format!("拖动进度失败: {err}"), cx),
    }
    runtime.player.update(cx, |player, cx| {
        player.position_ms = target_ms;
        cx.notify();
    });
    persist_player_progress(runtime, cx);
}

pub fn cycle_play_mode<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    runtime.player.update(cx, |player, cx| {
        player.cycle_mode();
        cx.notify();
    });
    persist_player_settings(runtime, cx);
    persist_player_runtime(runtime, cx);
}

pub fn toggle_playback<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let is_playing = runtime.player.read(cx).is_playing;
    if is_playing {
        match with_audio_bridge(runtime, |audio| audio.send(AudioCommand::Pause)) {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => auth::set_shell_error(runtime, Some(format!("暂停失败: {err}")), cx),
            Err(err) => auth::set_shell_error(runtime, Some(format!("暂停失败: {err}")), cx),
        }
        runtime.player.update(cx, |player, cx| {
            player.is_playing = false;
            cx.notify();
        });
        persist_player_runtime(runtime, cx);
        return;
    }

    let resumed = match with_audio_bridge(runtime, |audio| {
        let snapshot = audio.service().snapshot();
        if snapshot.source.is_some() {
            audio.send(AudioCommand::Play).map(|_| true)
        } else {
            Ok(false)
        }
    }) {
        Ok(Ok(resumed)) => resumed,
        Ok(Err(err)) => {
            auth::set_shell_error(runtime, Some(format!("恢复播放失败: {err}")), cx);
            false
        }
        Err(err) => {
            auth::set_shell_error(runtime, Some(format!("恢复播放失败: {err}")), cx);
            false
        }
    };

    if resumed {
        runtime.player.update(cx, |player, cx| {
            player.is_playing = true;
            cx.notify();
        });
        persist_player_runtime(runtime, cx);
        return;
    }

    if with_audio_bridge_or_error(runtime, cx, "播放失败", |_| {}).is_none() {
        return;
    }

    play_current(runtime, cx);
}

pub fn play_previous<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let mut target = None;
    runtime.player.update(cx, |player, _| {
        target = player.prev_index();
        player.position_ms = 0;
        player.duration_ms = 0;
    });
    if let Some(index) = target {
        start_playback_at(runtime, index, 0, true, cx);
    }
}

pub fn play_next<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let mut target = None;
    runtime.player.update(cx, |player, _| {
        target = player.next_index();
        player.position_ms = 0;
        player.duration_ms = 0;
    });
    if let Some(index) = target {
        start_playback_at(runtime, index, 0, true, cx);
    }
}

pub(super) fn play_current<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let player = runtime.player.read(cx).clone();
    let Some(current_index) = player.current_index else {
        return;
    };
    start_playback_at(runtime, current_index, player.position_ms, true, cx);
}

pub fn sync_audio_bridge<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    let mut ended = false;
    let mut forbidden = false;
    let mut last_error: Option<String> = None;

    let handled = with_audio_bridge(runtime, |bridge| {
        runtime.player.update(cx, |player, _| {
            for event in bridge.drain(player) {
                match event {
                    ame_audio::AudioEvent::TrackEnded => ended = true,
                    ame_audio::AudioEvent::Error(err) => {
                        if matches!(err, AudioError::HttpStatus { code: 403, .. }) {
                            forbidden = true;
                        }
                    }
                    _ => {}
                }
            }
        });
        last_error = bridge.last_error.take();
    });

    if let Err(err) = handled {
        set_shell_error_if_changed(runtime, format!("音频同步失败: {err}"), cx);
        return;
    }

    if forbidden && refresh_current_track_url_and_resume(runtime, cx) {
        return;
    }

    if let Some(err) = last_error {
        auth::set_shell_error(runtime, Some(err), cx);
    }

    if ended {
        let mut target = None;
        runtime.player.update(cx, |player, _| {
            target = player.next_index();
            player.position_ms = 0;
            player.duration_ms = 0;
        });
        if let Some(index) = target {
            start_playback_at(runtime, index, 0, true, cx);
        }
    }
}

pub fn prepare_app_exit<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    persist_player_settings(runtime, cx);
    persist_player_runtime(runtime, cx);
    persist_player_progress(runtime, cx);
    match with_audio_bridge(runtime, |audio| audio.send(AudioCommand::Stop)) {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => {
            auth::set_shell_error(runtime, Some(format!("退出前停止播放失败: {err}")), cx)
        }
        Err(err) => auth::set_shell_error(runtime, Some(format!("退出前停止播放失败: {err}")), cx),
    }
}
