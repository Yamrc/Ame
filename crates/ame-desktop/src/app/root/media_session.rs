use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use nekowg::{Context, Window};
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig,
    SeekDirection,
};
use tracing::{debug, warn};

#[cfg(target_os = "windows")]
use raw_window_handle::RawWindowHandle;

use crate::app::tray;
use crate::domain::player::{self, PlayerEntity, QueueItem};

use super::RootView;

const APP_DBUS_NAME: &str = "ame";
const APP_DISPLAY_NAME: &str = "Ame";
const PROGRESS_SYNC_INTERVAL: Duration = Duration::from_secs(1);
const SEEK_STEP_MS: u64 = 10_000;
const SEEK_IMMEDIATE_DELTA_MS: u64 = 2_000;

pub(super) struct MediaSessionManager {
    controls: Option<MediaControls>,
    event_rx: Receiver<MediaControlEvent>,
    last_metadata: Option<PublishedMetadata>,
    last_playback: Option<PublishedPlayback>,
    last_volume: Option<f32>,
    last_playback_sync_at: Option<Instant>,
    force_next_sync: bool,
    explicitly_stopped: bool,
    explicit_stop_track_id: Option<i64>,
}

impl MediaSessionManager {
    pub(super) fn new(window: &mut Window) -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        let controls = match create_media_controls(window, event_tx) {
            Ok(controls) => Some(controls),
            Err(err) => {
                warn!("media session initialization failed: {err}");
                None
            }
        };

        Self {
            controls,
            event_rx,
            last_metadata: None,
            last_playback: None,
            last_volume: None,
            last_playback_sync_at: None,
            force_next_sync: false,
            explicitly_stopped: false,
            explicit_stop_track_id: None,
        }
    }

    pub(super) fn drain_events(&mut self) -> Vec<MediaControlEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        if !events.is_empty() {
            self.force_next_sync = true;
        }
        events
    }

    pub(super) fn sync_player(&mut self, player: &PlayerEntity, now: Instant) {
        if self.controls.is_none() {
            return;
        }

        let metadata = metadata_from_player(player);
        let metadata_changed = metadata != self.last_metadata;
        if metadata_changed {
            if let Some(metadata) = metadata.as_ref() {
                self.publish_metadata(metadata);
            }
            self.last_metadata = metadata;
        }

        self.refresh_explicit_stop(player);
        let playback = playback_from_player(player, self.explicitly_stopped);
        if self.should_publish_playback(&playback, now, metadata_changed) {
            self.publish_playback(&playback, now);
        }

        self.publish_volume(player.volume);
        self.force_next_sync = false;
    }

    pub(super) fn publish_volume(&mut self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        if self.last_volume == Some(volume) {
            return;
        }

        if let Some(controls) = self.controls.as_mut() {
            publish_platform_volume(controls, volume);
        }
        self.last_volume = Some(volume);
    }

    pub(super) fn mark_stopped(&mut self, player: &PlayerEntity) {
        self.explicitly_stopped = true;
        self.explicit_stop_track_id = player.current_item().map(|item| item.id);
        self.publish_stopped();
    }

    pub(super) fn publish_stopped(&mut self) {
        let playback = PublishedPlayback::stopped();
        if let Some(controls) = self.controls.as_mut()
            && let Err(err) = controls.set_playback(playback.to_media_playback())
        {
            warn!("media session playback update failed: {err}");
        }
        self.last_playback = Some(playback);
        self.force_next_sync = false;
    }

    fn refresh_explicit_stop(&mut self, player: &PlayerEntity) {
        if !self.explicitly_stopped {
            return;
        }

        let current_track_id = player.current_item().map(|item| item.id);
        if player.is_playing
            || player.position_ms != 0
            || current_track_id != self.explicit_stop_track_id
        {
            self.explicitly_stopped = false;
            self.explicit_stop_track_id = None;
        }
    }

    pub(super) fn raise_main_window(&self, cx: &mut Context<RootView>) {
        tray::show_main_window(cx);
    }

    fn publish_metadata(&mut self, metadata: &PublishedMetadata) {
        let Some(controls) = self.controls.as_mut() else {
            return;
        };
        if let Err(err) = controls.set_metadata(metadata.to_media_metadata()) {
            warn!("media session metadata update failed: {err}");
        }
    }

    fn publish_playback(&mut self, playback: &PublishedPlayback, now: Instant) {
        let Some(controls) = self.controls.as_mut() else {
            return;
        };
        if let Err(err) = controls.set_playback(playback.to_media_playback()) {
            warn!("media session playback update failed: {err}");
        }
        self.last_playback = Some(playback.clone());
        self.last_playback_sync_at = Some(now);
    }

    fn should_publish_playback(
        &self,
        playback: &PublishedPlayback,
        now: Instant,
        metadata_changed: bool,
    ) -> bool {
        let Some(last) = self.last_playback.as_ref() else {
            return true;
        };
        if last == playback {
            return false;
        }
        if self.force_next_sync || metadata_changed {
            return true;
        }
        if last.track_id != playback.track_id || last.status != playback.status {
            return true;
        }
        if playback.status != PlaybackStatus::Playing {
            return true;
        }
        if playback.position_ms.abs_diff(last.position_ms) >= SEEK_IMMEDIATE_DELTA_MS {
            return true;
        }
        self.last_playback_sync_at
            .is_none_or(|last_sync| now.duration_since(last_sync) >= PROGRESS_SYNC_INTERVAL)
    }
}

impl RootView {
    pub(super) fn drain_media_session_events(&mut self, cx: &mut Context<Self>) {
        for event in self.media_session.drain_events() {
            self.handle_media_session_event(event, cx);
        }
    }

    pub(super) fn sync_media_session(&mut self, now: Instant, cx: &mut Context<Self>) {
        let player = self.env.player().read(cx).clone();
        self.media_session.sync_player(&player, now);
    }

    fn handle_media_session_event(&mut self, event: MediaControlEvent, cx: &mut Context<Self>) {
        match event {
            MediaControlEvent::Play => {
                if !self.runtime.player.read(cx).is_playing {
                    player::toggle_playback(&self.runtime, cx);
                }
            }
            MediaControlEvent::Pause => {
                if self.runtime.player.read(cx).is_playing {
                    player::toggle_playback(&self.runtime, cx);
                }
            }
            MediaControlEvent::Toggle => player::toggle_playback(&self.runtime, cx),
            MediaControlEvent::Next => player::play_next(&self.runtime, cx),
            MediaControlEvent::Previous => player::play_previous(&self.runtime, cx),
            MediaControlEvent::Stop => {
                player::stop_preserving_queue(&self.runtime, cx);
                let player = self.runtime.player.read(cx).clone();
                self.media_session.mark_stopped(&player);
            }
            MediaControlEvent::Seek(direction) => {
                self.seek_media_session_relative(direction, SEEK_STEP_MS, cx)
            }
            MediaControlEvent::SeekBy(direction, duration) => {
                self.seek_media_session_relative(
                    direction,
                    duration_to_millis_saturating(duration),
                    cx,
                );
            }
            MediaControlEvent::SetPosition(position) => {
                self.seek_media_session_absolute(duration_to_millis_saturating(position.0), cx);
            }
            MediaControlEvent::SetVolume(volume) => {
                let volume = volume.clamp(0.0, 1.0) as f32;
                player::set_volume_absolute(&self.runtime, volume, cx);
                self.media_session.publish_volume(volume);
            }
            MediaControlEvent::OpenUri(uri) => {
                debug!("ignoring media session OpenUri: {uri}");
            }
            MediaControlEvent::Raise => self.media_session.raise_main_window(cx),
            MediaControlEvent::Quit => {
                self.prepare_app_exit(cx);
                cx.quit();
            }
        }
    }

    fn seek_media_session_relative(
        &mut self,
        direction: SeekDirection,
        offset_ms: u64,
        cx: &mut Context<Self>,
    ) {
        let player = self.runtime.player.read(cx).clone();
        let target_ms = seek_target_ms(
            player.position_ms,
            effective_duration_ms(&player).unwrap_or(0),
            direction,
            offset_ms,
        );
        player::seek_to_position_ms(&self.runtime, target_ms, cx);
    }

    fn seek_media_session_absolute(&mut self, position_ms: u64, cx: &mut Context<Self>) {
        let player = self.runtime.player.read(cx).clone();
        let target_ms = clamp_position_ms(position_ms, effective_duration_ms(&player).unwrap_or(0));
        player::seek_to_position_ms(&self.runtime, target_ms, cx);
    }
}

fn create_media_controls(
    window: &mut Window,
    event_tx: Sender<MediaControlEvent>,
) -> Result<MediaControls, String> {
    let config = PlatformConfig {
        display_name: APP_DISPLAY_NAME,
        dbus_name: APP_DBUS_NAME,
        hwnd: platform_hwnd(window)?,
    };
    let mut controls = MediaControls::new(config).map_err(|err| err.to_string())?;
    controls
        .attach(move |event| {
            if event_tx.send(event).is_err() {
                debug!("media session event receiver dropped");
            }
        })
        .map_err(|err| err.to_string())?;
    Ok(controls)
}

#[cfg(target_os = "windows")]
fn platform_hwnd(window: &mut Window) -> Result<Option<*mut c_void>, String> {
    let handle =
        raw_window_handle::HasWindowHandle::window_handle(window).map_err(|err| err.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("expected Win32 window handle".to_string());
    };
    Ok(Some(handle.hwnd.get() as *mut c_void))
}

#[cfg(not(target_os = "windows"))]
fn platform_hwnd(_: &mut Window) -> Result<Option<*mut c_void>, String> {
    Ok(None)
}

#[cfg(target_os = "linux")]
fn publish_platform_volume(controls: &mut MediaControls, volume: f32) {
    if let Err(err) = controls.set_volume(f64::from(volume)) {
        warn!("media session volume update failed: {err}");
    }
}

#[cfg(not(target_os = "linux"))]
fn publish_platform_volume(_: &mut MediaControls, _: f32) {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishedMetadata {
    track_id: i64,
    title: String,
    artist: String,
    album: Option<String>,
    cover_url: Option<String>,
    duration_ms: Option<u64>,
}

impl PublishedMetadata {
    fn to_media_metadata(&self) -> MediaMetadata<'_> {
        MediaMetadata {
            title: Some(self.title.as_str()),
            album: self.album.as_deref(),
            artist: Some(self.artist.as_str()),
            cover_url: self.cover_url.as_deref(),
            duration: self.duration_ms.map(Duration::from_millis),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishedPlayback {
    track_id: Option<i64>,
    status: PlaybackStatus,
    position_ms: u64,
}

impl PublishedPlayback {
    fn stopped() -> Self {
        Self {
            track_id: None,
            status: PlaybackStatus::Stopped,
            position_ms: 0,
        }
    }

    fn to_media_playback(&self) -> MediaPlayback {
        match self.status {
            PlaybackStatus::Stopped => MediaPlayback::Stopped,
            PlaybackStatus::Paused => MediaPlayback::Paused {
                progress: Some(MediaPosition(Duration::from_millis(self.position_ms))),
            },
            PlaybackStatus::Playing => MediaPlayback::Playing {
                progress: Some(MediaPosition(Duration::from_millis(self.position_ms))),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackStatus {
    Stopped,
    Paused,
    Playing,
}

fn metadata_from_player(player: &PlayerEntity) -> Option<PublishedMetadata> {
    let item = player.current_item()?;
    Some(PublishedMetadata {
        track_id: item.id,
        title: item.name.clone(),
        artist: item.artist.clone(),
        album: item.album.clone(),
        cover_url: item.cover_url.clone(),
        duration_ms: metadata_duration_ms(player, item),
    })
}

fn playback_from_player(player: &PlayerEntity, explicitly_stopped: bool) -> PublishedPlayback {
    if explicitly_stopped {
        return PublishedPlayback::stopped();
    }

    let Some(item) = player.current_item() else {
        return PublishedPlayback::stopped();
    };

    let duration_ms = metadata_duration_ms(player, item).unwrap_or(0);
    PublishedPlayback {
        track_id: Some(item.id),
        status: if player.is_playing {
            PlaybackStatus::Playing
        } else {
            PlaybackStatus::Paused
        },
        position_ms: clamp_position_ms(player.position_ms, duration_ms),
    }
}

fn effective_duration_ms(player: &PlayerEntity) -> Option<u64> {
    metadata_duration_ms(player, player.current_item()?)
}

fn metadata_duration_ms(player: &PlayerEntity, item: &QueueItem) -> Option<u64> {
    if player.duration_ms != 0 {
        Some(player.duration_ms)
    } else {
        item.duration_ms
    }
}

fn seek_target_ms(
    position_ms: u64,
    duration_ms: u64,
    direction: SeekDirection,
    offset_ms: u64,
) -> u64 {
    let target = match direction {
        SeekDirection::Forward => position_ms.saturating_add(offset_ms),
        SeekDirection::Backward => position_ms.saturating_sub(offset_ms),
    };
    clamp_position_ms(target, duration_ms)
}

fn clamp_position_ms(position_ms: u64, duration_ms: u64) -> u64 {
    if duration_ms == 0 {
        position_ms
    } else {
        position_ms.min(duration_ms)
    }
}

fn duration_to_millis_saturating(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue_item() -> QueueItem {
        QueueItem {
            id: 42,
            name: "Song".into(),
            alias: None,
            artist: "Artist".into(),
            album: Some("Album".into()),
            duration_ms: Some(123_000),
            cover_url: Some("https://example.test/cover.jpg".into()),
            source_url: None,
        }
    }

    fn player_with_current() -> PlayerEntity {
        let mut player = PlayerEntity::default();
        player.duration_ms = 240_000;
        player.enqueue(queue_item());
        player
    }

    #[test]
    fn metadata_maps_current_queue_item() {
        let player = player_with_current();
        let metadata = metadata_from_player(&player).expect("metadata");

        assert_eq!(metadata.track_id, 42);
        assert_eq!(metadata.title, "Song");
        assert_eq!(metadata.artist, "Artist");
        assert_eq!(metadata.album.as_deref(), Some("Album"));
        assert_eq!(
            metadata.cover_url.as_deref(),
            Some("https://example.test/cover.jpg")
        );
        assert_eq!(metadata.duration_ms, Some(240_000));
    }

    #[test]
    fn metadata_uses_queue_duration_when_player_duration_is_zero() {
        let mut player = player_with_current();
        player.duration_ms = 0;

        let metadata = metadata_from_player(&player).expect("metadata");

        assert_eq!(metadata.duration_ms, Some(123_000));
    }

    #[test]
    fn playback_maps_empty_player_to_stopped() {
        let playback = playback_from_player(&PlayerEntity::default(), false);

        assert_eq!(playback.status, PlaybackStatus::Stopped);
        assert_eq!(playback.track_id, None);
        assert_eq!(playback.position_ms, 0);
    }

    #[test]
    fn playback_maps_current_player_state() {
        let mut player = player_with_current();
        player.is_playing = true;
        player.position_ms = 245_000;
        player.duration_ms = 240_000;

        let playback = playback_from_player(&player, false);

        assert_eq!(playback.status, PlaybackStatus::Playing);
        assert_eq!(playback.track_id, Some(42));
        assert_eq!(playback.position_ms, 240_000);
    }

    #[test]
    fn seek_target_saturates_and_clamps() {
        assert_eq!(
            seek_target_ms(5_000, 60_000, SeekDirection::Backward, 10_000),
            0
        );
        assert_eq!(
            seek_target_ms(55_000, 60_000, SeekDirection::Forward, 10_000),
            60_000
        );
        assert_eq!(
            seek_target_ms(55_000, 0, SeekDirection::Forward, 10_000),
            65_000
        );
    }

    #[test]
    fn duration_to_millis_saturates_to_u64() {
        assert_eq!(duration_to_millis_saturating(Duration::from_millis(42)), 42);
        assert_eq!(
            duration_to_millis_saturating(Duration::from_secs(u64::MAX)),
            u64::MAX
        );
    }
}
