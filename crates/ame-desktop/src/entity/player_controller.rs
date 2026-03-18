use std::time::{Duration, Instant};

use ame_audio::{AudioCommand, AudioError, AudioRuntimeHandle, SeekTarget, SourceSpec};
use nekowg::Context;

use crate::action::{player_actions, queue_actions};
use crate::entity::audio_bridge::AudioBridgeEntity;
use crate::entity::player::QueueItem;
use crate::entity::runtime::{
    AppRuntime, KEY_PLAYER_CURRENT_INDEX, KEY_PLAYER_DURATION_MS, KEY_PLAYER_MODE,
    KEY_PLAYER_POSITION_MS, KEY_PLAYER_QUEUE, KEY_PLAYER_VOLUME, KEY_PLAYER_WAS_PLAYING,
    KEY_WINDOW_CLOSE_BEHAVIOR, PersistedQueueItem,
};
use crate::entity::services::auth::{self, AuthLevel};
use crate::view::{playlist, search};

const PROGRESS_PERSIST_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct QueueTrackInput {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub cover_url: Option<String>,
}

impl From<search::SearchSong> for QueueTrackInput {
    fn from(value: search::SearchSong) -> Self {
        Self {
            id: value.id,
            name: value.name,
            alias: value.alias,
            artists: value.artists,
            cover_url: value.cover_url,
        }
    }
}

impl From<playlist::PlaylistTrackRow> for QueueTrackInput {
    fn from(value: playlist::PlaylistTrackRow) -> Self {
        Self {
            id: value.id,
            name: value.name,
            alias: value.alias,
            artists: value.artists,
            cover_url: value.cover_url,
        }
    }
}

impl From<crate::action::library_actions::PlaylistTrackItem> for QueueTrackInput {
    fn from(value: crate::action::library_actions::PlaylistTrackItem) -> Self {
        Self {
            id: value.id,
            name: value.name,
            alias: value.alias,
            artists: value.artists,
            cover_url: value.cover_url,
        }
    }
}

impl From<crate::action::library_actions::DailyTrackItem> for QueueTrackInput {
    fn from(value: crate::action::library_actions::DailyTrackItem) -> Self {
        Self {
            id: value.id,
            name: value.name,
            alias: value.alias,
            artists: value.artists,
            cover_url: value.cover_url,
        }
    }
}

impl From<crate::action::library_actions::FmTrackItem> for QueueTrackInput {
    fn from(value: crate::action::library_actions::FmTrackItem) -> Self {
        Self {
            id: value.id,
            name: value.name,
            alias: value.alias,
            artists: value.artists,
            cover_url: value.cover_url,
        }
    }
}

pub struct PlayerController {
    runtime: AppRuntime,
    audio_bridge: Option<AudioBridgeEntity>,
    _audio_runtime: Option<AudioRuntimeHandle>,
    last_progress_persist_at: Instant,
}

impl PlayerController {
    pub fn new(
        runtime: AppRuntime,
        audio_bridge: Option<AudioBridgeEntity>,
        audio_runtime: Option<AudioRuntimeHandle>,
        cx: &mut Context<Self>,
    ) -> Self {
        if let Some(audio) = &audio_bridge {
            let _ = audio.send(AudioCommand::SetVolume(runtime.player.read(cx).volume));
        }
        Self {
            runtime,
            audio_bridge,
            _audio_runtime: audio_runtime,
            last_progress_persist_at: Instant::now(),
        }
    }

    pub fn set_volume_absolute(&mut self, volume: f32, cx: &mut Context<Self>) {
        let volume = volume.clamp(0.0, 1.0);
        self.runtime.player.update(cx, |player, cx| {
            player.set_volume(volume);
            cx.notify();
        });
        if let Some(audio) = &self.audio_bridge {
            let _ = audio.send(AudioCommand::SetVolume(volume));
        }
        self.persist_player_settings(cx);
    }

    pub fn preview_seek_ratio(&mut self, ratio: f32, cx: &mut Context<Self>) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.runtime.player.update(cx, |player, cx| {
            let duration = player.duration_ms.max(1);
            player.position_ms = ((duration as f32) * ratio) as u64;
            cx.notify();
        });
    }

    pub fn commit_seek_ratio(&mut self, ratio: f32, cx: &mut Context<Self>) {
        let ratio = ratio.clamp(0.0, 1.0);
        let duration_ms = self.runtime.player.read(cx).duration_ms.max(1);
        let target_ms = ((duration_ms as f32) * ratio) as u64;
        if let Some(audio) = &self.audio_bridge {
            let _ = audio.send(AudioCommand::Seek(SeekTarget::ms(target_ms)));
        }
        self.runtime.player.update(cx, |player, cx| {
            player.position_ms = target_ms;
            cx.notify();
        });
        self.persist_player_progress(cx);
    }

    pub fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        let is_playing = self.runtime.player.read(cx).is_playing;
        if is_playing {
            if let Some(audio) = &self.audio_bridge {
                let _ = audio.send(AudioCommand::Pause);
            }
            self.runtime.player.update(cx, |player, cx| {
                player.is_playing = false;
                cx.notify();
            });
            self.persist_player_runtime(cx);
            return;
        }

        if let Some(audio) = &self.audio_bridge {
            let snapshot = audio.service().snapshot();
            if snapshot.source.is_some() && audio.send(AudioCommand::Play).is_ok() {
                self.runtime.player.update(cx, |player, cx| {
                    player.is_playing = true;
                    cx.notify();
                });
                self.persist_player_runtime(cx);
                return;
            }
        }

        self.play_current(cx);
    }

    pub fn play_previous(&mut self, cx: &mut Context<Self>) {
        let mut target = None;
        self.runtime.player.update(cx, |player, _| {
            target = player.prev_index();
            player.position_ms = 0;
            player.duration_ms = 0;
        });
        if let Some(index) = target {
            self.start_playback_at(index, 0, true, cx);
        }
    }

    pub fn play_next(&mut self, cx: &mut Context<Self>) {
        let mut target = None;
        self.runtime.player.update(cx, |player, _| {
            target = player.next_index();
            player.position_ms = 0;
            player.duration_ms = 0;
        });
        if let Some(index) = target {
            self.start_playback_at(index, 0, true, cx);
        }
    }

    pub fn cycle_play_mode(&mut self, cx: &mut Context<Self>) {
        self.runtime.player.update(cx, |player, cx| {
            player.cycle_mode();
            cx.notify();
        });
        self.persist_player_settings(cx);
        self.persist_player_runtime(cx);
    }

    pub fn play_current(&mut self, cx: &mut Context<Self>) {
        let player = self.runtime.player.read(cx).clone();
        let Some(current_index) = player.current_index else {
            return;
        };
        self.start_playback_at(current_index, player.position_ms, true, cx);
    }

    pub fn enqueue_track(
        &mut self,
        track: QueueTrackInput,
        autoplay: bool,
        cx: &mut Context<Self>,
    ) {
        if let Some(existing_index) =
            queue_actions::index_of(self.runtime.player.read(cx), track.id)
        {
            if autoplay {
                self.start_playback_at(existing_index, 0, true, cx);
            }
            return;
        }

        let Some(cookie) = auth::ensure_auth_cookie(&self.runtime, AuthLevel::Guest, cx) else {
            return;
        };
        let metadata =
            player_actions::fetch_track_metadata_blocking(track.id, Some(cookie.as_str())).ok();
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

        let mut inserted_index = None;
        self.runtime.player.update(cx, |player, _| {
            player.enqueue(QueueItem {
                id: track.id,
                name: track.name.clone(),
                alias,
                artist: artists,
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
                self.start_playback_at(index, 0, true, cx);
            } else {
                self.persist_player_runtime(cx);
            }
        } else {
            self.persist_player_runtime(cx);
        }
    }

    pub fn replace_queue(
        &mut self,
        tracks: Vec<QueueTrackInput>,
        start_index: usize,
        cx: &mut Context<Self>,
    ) {
        if tracks.is_empty() {
            auth::push_shell_error(&self.runtime, "歌单为空，无法替换队列".to_string(), cx);
            return;
        }

        self.runtime.player.update(cx, |player, _| {
            player.set_queue(
                tracks
                    .iter()
                    .map(|track| QueueItem {
                        id: track.id,
                        name: track.name.clone(),
                        alias: track.alias.clone(),
                        artist: track.artists.clone(),
                        cover_url: track.cover_url.clone(),
                        source_url: None,
                    })
                    .collect(),
            );
            player.current_index = Some(start_index.min(player.queue.len().saturating_sub(1)));
            player.position_ms = 0;
            player.duration_ms = 0;
        });

        if !self.start_playback_at(start_index, 0, true, cx) {
            self.persist_player_runtime(cx);
            self.persist_player_progress(cx);
        }
    }

    pub fn play_queue_item(&mut self, track_id: i64, cx: &mut Context<Self>) {
        let Some(index) = queue_actions::index_of(self.runtime.player.read(cx), track_id) else {
            return;
        };
        self.start_playback_at(index, 0, true, cx);
    }

    pub fn remove_queue_item(&mut self, track_id: i64, cx: &mut Context<Self>) {
        self.runtime.player.update(cx, |player, cx| {
            queue_actions::remove_by_id(player, track_id);
            cx.notify();
        });
        self.persist_player_runtime(cx);
    }

    pub fn clear_queue(&mut self, cx: &mut Context<Self>) {
        self.runtime.player.update(cx, |player, cx| {
            queue_actions::clear(player);
            player.is_playing = false;
            player.position_ms = 0;
            player.duration_ms = 0;
            cx.notify();
        });
        if let Some(audio) = &self.audio_bridge {
            let _ = audio.send(AudioCommand::Stop);
        }
        self.persist_player_runtime(cx);
        self.persist_player_progress(cx);
    }

    pub fn prepare_app_exit(&mut self, cx: &mut Context<Self>) {
        self.persist_player_settings(cx);
        self.persist_player_runtime(cx);
        self.persist_player_progress(cx);
        if let Some(audio) = &self.audio_bridge {
            let _ = audio.send(AudioCommand::Stop);
        }
    }

    pub fn sync_audio_bridge(&mut self, cx: &mut Context<Self>) {
        let mut ended = false;
        let mut forbidden = false;
        let mut last_error: Option<String> = None;
        if let Some(bridge) = self.audio_bridge.as_mut() {
            self.runtime.player.update(cx, |player, _| {
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
        }

        if forbidden && self.refresh_current_track_url_and_resume(cx) {
            return;
        }

        if let Some(err) = last_error {
            auth::set_shell_error(&self.runtime, Some(err), cx);
        }

        if ended {
            let mut target = None;
            self.runtime.player.update(cx, |player, _| {
                target = player.next_index();
                player.position_ms = 0;
                player.duration_ms = 0;
            });
            if let Some(index) = target {
                self.start_playback_at(index, 0, true, cx);
            }
        }
    }

    pub fn persist_progress_by_interval(&mut self, now: Instant, cx: &mut Context<Self>) {
        if now.duration_since(self.last_progress_persist_at) < PROGRESS_PERSIST_INTERVAL {
            return;
        }
        self.last_progress_persist_at = now;
        self.persist_player_progress(cx);
    }

    fn prepare_track_source(
        &mut self,
        track_id: i64,
        queue_index: usize,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let cookie = auth::ensure_auth_cookie(&self.runtime, AuthLevel::Guest, cx)?;
        let source_url =
            match player_actions::fetch_track_url_blocking(track_id, Some(cookie.as_str())) {
                Ok(url) => url,
                Err(err) => {
                    auth::set_shell_error(
                        &self.runtime,
                        Some(format!("获取播放地址失败: {err}")),
                        cx,
                    );
                    return None;
                }
            };

        self.runtime.player.update(cx, |player, _| {
            if let Some(item) = player.queue.get_mut(queue_index) {
                item.source_url = Some(source_url.clone());
            }
        });
        Some(source_url)
    }

    fn start_playback_at(
        &mut self,
        queue_index: usize,
        start_ms: u64,
        autoplay: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let snapshot = self.runtime.player.read(cx).clone();
        let Some(item) = snapshot.queue.get(queue_index).cloned() else {
            return false;
        };

        let Some(source_url) = self.prepare_track_source(item.id, queue_index, cx) else {
            self.persist_player_runtime(cx);
            return false;
        };

        if let Some(audio) = &self.audio_bridge
            && let Err(err) = audio.send(AudioCommand::Open {
                source: SourceSpec::network(source_url),
                start_ms,
                autoplay,
            })
        {
            auth::set_shell_error(&self.runtime, Some(format!("播放失败: {err}")), cx);
            return false;
        }

        self.runtime.player.update(cx, |player, cx| {
            player.current_index = Some(queue_index);
            player.is_playing = autoplay;
            cx.notify();
        });
        self.persist_player_runtime(cx);
        true
    }

    fn refresh_current_track_url_and_resume(&mut self, cx: &mut Context<Self>) -> bool {
        let player_snapshot = self.runtime.player.read(cx).clone();
        let Some(current_index) = player_snapshot.current_index else {
            return false;
        };
        let Some(current_item) = player_snapshot.queue.get(current_index).cloned() else {
            return false;
        };

        let Some(url) = self.prepare_track_source(current_item.id, current_index, cx) else {
            auth::set_shell_error(&self.runtime, Some("刷新播放地址失败".to_string()), cx);
            return false;
        };

        if let Some(audio) = &self.audio_bridge {
            let position = audio.service().snapshot().position_ms;
            if let Err(err) = audio.send(AudioCommand::Open {
                source: SourceSpec::network(url),
                start_ms: position,
                autoplay: true,
            }) {
                auth::set_shell_error(&self.runtime, Some(format!("刷新播放失败: {err}")), cx);
                return false;
            }
        }

        self.runtime.player.update(cx, |player, cx| {
            player.is_playing = true;
            cx.notify();
        });
        self.persist_player_runtime(cx);
        true
    }

    fn persist_player_settings(&mut self, cx: &mut Context<Self>) {
        let Some(settings) = self.runtime.services.settings_store.as_ref() else {
            return;
        };
        let player = self.runtime.player.read(cx).clone();
        let close_behavior = self.runtime.shell.read(cx).close_behavior;
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
            auth::push_shell_error(&self.runtime, err, cx);
        }
    }

    fn persist_player_runtime(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.runtime.services.state_store.as_ref() else {
            return;
        };
        let player = self.runtime.player.read(cx).clone();
        let mut errors = Vec::new();
        let queue = player
            .queue
            .iter()
            .map(|item| PersistedQueueItem {
                id: item.id,
                name: item.name.clone(),
                alias: item.alias.clone(),
                artist: item.artist.clone(),
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
            auth::push_shell_error(&self.runtime, err, cx);
        }
    }

    fn persist_player_progress(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.runtime.services.state_store.as_ref() else {
            return;
        };
        let player = self.runtime.player.read(cx).clone();
        let mut errors = Vec::new();
        if let Err(err) = state.set(KEY_PLAYER_POSITION_MS, &player.position_ms) {
            errors.push(format!("保存播放进度失败: {err}"));
        }
        if let Err(err) = state.set(KEY_PLAYER_DURATION_MS, &player.duration_ms) {
            errors.push(format!("保存播放时长失败: {err}"));
        }
        for err in errors {
            auth::push_shell_error(&self.runtime, err, cx);
        }
    }
}
