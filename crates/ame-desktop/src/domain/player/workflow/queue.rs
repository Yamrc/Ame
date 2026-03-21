use ame_audio::AudioCommand;
use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::player;
use crate::domain::player::QueueItem;
use crate::domain::session as auth;
use crate::domain::session::AuthLevel;

use super::bridge::with_audio_bridge;
use super::persist::{persist_player_progress, persist_player_runtime};
use super::playback::start_playback_at;
use super::types::QueueTrackInput;

pub fn enqueue_track<T>(
    runtime: &AppRuntime,
    track: QueueTrackInput,
    autoplay: bool,
    cx: &mut Context<T>,
) {
    if let Some(existing_index) = runtime.player.read(cx).index_of_id(track.id) {
        if autoplay {
            start_playback_at(runtime, existing_index, 0, true, cx);
        }
        return;
    }

    let Some(cookie) = auth::ensure_auth_cookie(runtime, AuthLevel::Guest, cx) else {
        return;
    };
    let metadata = match player::fetch_track_metadata_blocking(track.id, Some(cookie.as_str())) {
        Ok(meta) => Some(meta),
        Err(err) => {
            auth::push_shell_error(runtime, format!("Failed to fetch track details: {err}"), cx);
            None
        }
    };

    let artists = metadata
        .as_ref()
        .map(|meta| meta.artists.clone())
        .unwrap_or_else(|| track.artists.clone());
    let alias = metadata
        .as_ref()
        .and_then(|meta| meta.alias.clone())
        .or(track.alias.clone());
    let cover_url = metadata
        .as_ref()
        .and_then(|meta| meta.cover_url.clone())
        .or(track.cover_url.clone());
    let album = metadata
        .as_ref()
        .and_then(|meta| meta.album.clone())
        .or(track.album.clone());
    let duration_ms = metadata
        .as_ref()
        .and_then(|meta| meta.duration_ms)
        .or(track.duration_ms);

    let mut inserted_index = None;
    runtime.player.update(cx, |player, _| {
        player.enqueue(QueueItem {
            id: track.id,
            name: track.name.clone(),
            alias,
            artist: artists,
            album,
            duration_ms,
            cover_url,
            source_url: None,
        });
        inserted_index = player.queue.len().checked_sub(1);
        if autoplay {
            player.current_index = inserted_index;
        }
    });

    if autoplay {
        if let Some(index) = inserted_index {
            start_playback_at(runtime, index, 0, true, cx);
        } else {
            persist_player_runtime(runtime, cx);
        }
    } else {
        persist_player_runtime(runtime, cx);
    }
}

pub fn replace_queue<T>(
    runtime: &AppRuntime,
    tracks: Vec<QueueTrackInput>,
    start_index: usize,
    cx: &mut Context<T>,
) {
    if tracks.is_empty() {
        auth::push_shell_error(
            runtime,
            "Playlist is empty and cannot replace the queue".to_string(),
            cx,
        );
        return;
    }

    runtime.player.update(cx, |player, _| {
        player.set_queue(
            tracks
                .iter()
                .map(|track| QueueItem {
                    id: track.id,
                    name: track.name.clone(),
                    alias: track.alias.clone(),
                    artist: track.artists.clone(),
                    album: track.album.clone(),
                    duration_ms: track.duration_ms,
                    cover_url: track.cover_url.clone(),
                    source_url: None,
                })
                .collect(),
        );
        player.current_index = Some(start_index.min(player.queue.len().saturating_sub(1)));
        player.position_ms = 0;
        player.duration_ms = 0;
    });

    if !start_playback_at(runtime, start_index, 0, true, cx) {
        persist_player_runtime(runtime, cx);
        persist_player_progress(runtime, cx);
    }
}

pub fn play_queue_item<T>(runtime: &AppRuntime, track_id: i64, cx: &mut Context<T>) {
    let Some(index) = runtime.player.read(cx).index_of_id(track_id) else {
        return;
    };
    start_playback_at(runtime, index, 0, true, cx);
}

pub fn remove_queue_item<T>(runtime: &AppRuntime, track_id: i64, cx: &mut Context<T>) {
    runtime.player.update(cx, |player, cx| {
        if let Some(index) = player.index_of_id(track_id) {
            player.remove_at(index);
        }
        cx.notify();
    });
    persist_player_runtime(runtime, cx);
}

pub fn clear_queue<T>(runtime: &AppRuntime, cx: &mut Context<T>) {
    runtime.player.update(cx, |player, cx| {
        player.clear();
        player.is_playing = false;
        player.position_ms = 0;
        player.duration_ms = 0;
        cx.notify();
    });
    match with_audio_bridge(runtime, |audio| audio.send(AudioCommand::Stop)) {
        Ok(Ok(_)) => {}
        Ok(Err(err)) => {
            auth::push_shell_error(runtime, format!("Failed to stop playback: {err}"), cx)
        }
        Err(err) => auth::push_shell_error(runtime, format!("Failed to stop playback: {err}"), cx),
    }
    persist_player_runtime(runtime, cx);
    persist_player_progress(runtime, cx);
}
