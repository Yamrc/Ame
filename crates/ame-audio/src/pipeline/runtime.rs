use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, mpsc};
use std::time::{Duration, Instant};

use ringbuf::traits::Split;
use tracing::warn;

use crate::backend::{
    BackendNotification, OpenStreamRequest, OutputSession, PlaybackDrainState, SampleRing,
    backend_for_kind,
};
use crate::command::AudioCommand;
use crate::config::{AudioConfig, RuntimeConfigPatch};
use crate::decoder::{DecoderNotification, DecoderSpawnRequest, spawn_decoder};
use crate::error::{AudioError, Result};
use crate::event::AudioEvent;
use crate::service::SubscriptionHub;
use crate::snapshot::AudioSnapshot;
use crate::source::{DefaultSourceFactory, SeekTarget, SourceFactory, SourceSpec};
use crate::{EngineState, OutputBackendKind};

pub(crate) struct AudioRuntime {
    config: AudioConfig,
    state: EngineState,
    command_rx: mpsc::Receiver<AudioCommand>,
    event_hub: SubscriptionHub<AudioEvent>,
    snapshot_hub: SubscriptionHub<AudioSnapshot>,
    latest_snapshot: Arc<RwLock<AudioSnapshot>>,
    source_factory: Arc<dyn SourceFactory>,
    playback: Option<PlaybackPipeline>,
    current_source: Option<SourceSpec>,
    duration_ms: u64,
    position_base_ms: u64,
    playing_anchor: Option<Instant>,
    snapshot_interval: Duration,
    decoder_notify_tx: mpsc::Sender<DecoderNotification>,
    decoder_notify_rx: mpsc::Receiver<DecoderNotification>,
    backend_notify_tx: mpsc::Sender<BackendNotification>,
    backend_notify_rx: mpsc::Receiver<BackendNotification>,
    next_stream_id: u64,
}

struct PlaybackPipeline {
    cancel: Arc<AtomicBool>,
    decoder_thread: Option<std::thread::JoinHandle<()>>,
    output: Box<dyn OutputSession>,
    backend: OutputBackendKind,
    device_name: String,
    stream_id: u64,
}

impl PlaybackPipeline {
    fn stop(mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.output.pause();
        if let Some(thread) = self.decoder_thread.take() {
            let _ = thread.join();
        }
    }
}

impl AudioRuntime {
    pub(crate) fn new(
        config: AudioConfig,
        command_rx: mpsc::Receiver<AudioCommand>,
        event_hub: SubscriptionHub<AudioEvent>,
        snapshot_hub: SubscriptionHub<AudioSnapshot>,
        latest_snapshot: Arc<RwLock<AudioSnapshot>>,
    ) -> Result<Self> {
        let source_factory = Arc::new(DefaultSourceFactory::new(config.network.clone())?);
        let snapshot_interval = Duration::from_secs_f32(1.0 / config.snapshot_hz.max(1) as f32);
        let (decoder_notify_tx, decoder_notify_rx) = mpsc::channel();
        let (backend_notify_tx, backend_notify_rx) = mpsc::channel();

        Ok(Self {
            config,
            state: EngineState::Idle,
            command_rx,
            event_hub,
            snapshot_hub,
            latest_snapshot,
            source_factory,
            playback: None,
            current_source: None,
            duration_ms: 0,
            position_base_ms: 0,
            playing_anchor: None,
            snapshot_interval,
            decoder_notify_tx,
            decoder_notify_rx,
            backend_notify_tx,
            backend_notify_rx,
            next_stream_id: 1,
        })
    }

    pub(crate) fn run(mut self) {
        self.publish_snapshot();
        let mut last_snapshot = Instant::now();

        loop {
            self.drain_decoder_notifications();
            self.drain_backend_notifications();

            match self.command_rx.recv_timeout(Duration::from_millis(10)) {
                Ok(AudioCommand::Shutdown) => break,
                Ok(command) => {
                    if let Err(err) = self.handle_command(command) {
                        if matches!(err, AudioError::InvalidStateTransition { .. }) {
                            self.event_hub.publish(AudioEvent::Error(err));
                        } else {
                            self.publish_error(err);
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }

            if last_snapshot.elapsed() >= self.snapshot_interval {
                self.publish_snapshot();
                last_snapshot = Instant::now();
            }
        }

        self.stop_playback();
        let _ = self.transition_to(EngineState::Stopped);
        self.publish_snapshot();
    }

    fn handle_command(&mut self, command: AudioCommand) -> Result<()> {
        match command {
            AudioCommand::Open {
                source,
                start_ms,
                autoplay,
            } => self.start_source(source, start_ms, autoplay),
            AudioCommand::Play => self.resume(),
            AudioCommand::Pause => self.pause(),
            AudioCommand::Stop => {
                self.stop_playback();
                self.current_source = None;
                self.duration_ms = 0;
                self.position_base_ms = 0;
                self.playing_anchor = None;
                self.transition_to(EngineState::Stopped)
            }
            AudioCommand::Seek(target) => self.seek(target),
            AudioCommand::SetVolume(volume) => {
                self.config.volume = volume.clamp(0.0, 1.0);
                if let Some(playback) = &self.playback {
                    playback.output.set_volume(self.config.volume);
                }
                self.publish_snapshot();
                Ok(())
            }
            AudioCommand::SwitchBackend(kind) => self.switch_backend(kind),
            AudioCommand::SwitchDevice(device) => {
                self.config.preferred_device = device;
                self.restart_current_source()
            }
            AudioCommand::UpdateConfig(patch) => self.update_config(patch),
            AudioCommand::Shutdown => Ok(()),
        }
    }

    fn start_source(&mut self, source: SourceSpec, start_ms: u64, autoplay: bool) -> Result<()> {
        self.transition_to(EngineState::Loading)?;
        self.stop_playback();

        let stream_id = self.next_stream_id;
        self.next_stream_id = self.next_stream_id.wrapping_add(1);

        let backend = backend_for_kind(self.config.backend)?;
        let ring_capacity = 48_000 * 2 * 2;
        let ring = SampleRing::new(ring_capacity);
        let (producer, consumer) = ring.split();
        let drain_state = Arc::new(PlaybackDrainState::default());

        let output = backend.open_stream(OpenStreamRequest {
            stream_id,
            preferred_device: self.config.preferred_device.clone(),
            volume: self.config.volume,
            consumer,
            event_tx: self.backend_notify_tx.clone(),
            drain_state: Arc::clone(&drain_state),
        })?;

        if autoplay {
            output.play()?;
        } else {
            output.pause()?;
        }

        let sample_rate = output.sample_rate();
        let channels = output.channels() as usize;
        let cancel = Arc::new(AtomicBool::new(false));

        let decoder = spawn_decoder(DecoderSpawnRequest {
            source_factory: Arc::clone(&self.source_factory),
            source_spec: source.clone(),
            target_sample_rate: sample_rate,
            target_channels: channels,
            start_ms,
            quality: self.config.resample_quality,
            cancel: Arc::clone(&cancel),
            notification_tx: self.decoder_notify_tx.clone(),
            producer,
            drain_state,
        });

        let device_name = output.device_name();
        self.playback = Some(PlaybackPipeline {
            cancel,
            decoder_thread: Some(decoder),
            output,
            backend: self.config.backend,
            device_name: device_name.clone(),
            stream_id,
        });
        self.current_source = Some(source);
        self.position_base_ms = start_ms;
        self.playing_anchor = if autoplay { Some(Instant::now()) } else { None };

        self.transition_to(if autoplay {
            EngineState::Playing
        } else {
            EngineState::Ready
        })?;

        self.event_hub.publish(AudioEvent::DeviceChanged {
            backend: self.config.backend,
            device: Some(device_name),
        });

        self.publish_snapshot();
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        match self.state {
            EngineState::Playing => {
                if let Some(playback) = &self.playback {
                    playback.output.pause()?;
                }
                self.position_base_ms = self.current_position_ms();
                self.playing_anchor = None;
                self.transition_to(EngineState::Paused)?;
                self.publish_snapshot();
                Ok(())
            }
            _ => Err(AudioError::InvalidStateTransition {
                from: self.state,
                to: EngineState::Paused,
            }),
        }
    }

    fn resume(&mut self) -> Result<()> {
        match self.state {
            EngineState::Ready | EngineState::Paused => {
                if let Some(playback) = &self.playback {
                    playback.output.play()?;
                }
                self.playing_anchor = Some(Instant::now());
                self.transition_to(EngineState::Playing)?;
                self.publish_snapshot();
                Ok(())
            }
            _ => Err(AudioError::InvalidStateTransition {
                from: self.state,
                to: EngineState::Playing,
            }),
        }
    }

    fn seek(&mut self, target: SeekTarget) -> Result<()> {
        let source = self
            .current_source
            .clone()
            .ok_or(AudioError::ConfigInvalid {
                reason: "no source loaded".into(),
            })?;

        let target_ms = target.to_millis();
        let was_playing = self.state == EngineState::Playing;
        match self.start_source(source.clone(), target_ms, was_playing) {
            Ok(()) => {}
            Err(AudioError::UnsupportedSeek) if target_ms > 0 => {
                self.start_source(source, 0, was_playing)?;
            }
            Err(err) => return Err(err),
        }

        if !was_playing {
            self.transition_to(EngineState::Paused)?;
        }

        self.publish_snapshot();
        Ok(())
    }

    fn switch_backend(&mut self, kind: OutputBackendKind) -> Result<()> {
        self.config.backend = kind;
        self.restart_current_source()
    }

    fn update_config(&mut self, patch: RuntimeConfigPatch) -> Result<()> {
        let requires_rebuild = patch.backend.is_some()
            || patch.preferred_device.is_some()
            || patch.resample_quality.is_some()
            || patch.network.is_some();

        let rebuild_factory = patch.network.is_some();
        self.config.apply_patch(patch);
        self.snapshot_interval =
            Duration::from_secs_f32(1.0 / self.config.snapshot_hz.max(1) as f32);

        if rebuild_factory {
            self.source_factory = Arc::new(DefaultSourceFactory::new(self.config.network.clone())?);
        }

        if requires_rebuild {
            self.restart_current_source()?;
        } else if let Some(playback) = &self.playback {
            playback.output.set_volume(self.config.volume);
        }

        self.publish_snapshot();
        Ok(())
    }

    fn restart_current_source(&mut self) -> Result<()> {
        let Some(source) = self.current_source.clone() else {
            return Ok(());
        };

        let playing = self.state == EngineState::Playing;
        let position = self.current_position_ms();

        self.start_source(source, position, playing)?;
        if !playing {
            self.transition_to(EngineState::Paused)?;
        }

        Ok(())
    }

    fn stop_playback(&mut self) {
        if let Some(playback) = self.playback.take() {
            playback.stop();
        }
    }

    fn drain_decoder_notifications(&mut self) {
        loop {
            match self.decoder_notify_rx.try_recv() {
                Ok(DecoderNotification::Duration(duration_ms)) => {
                    self.duration_ms = duration_ms;
                }
                Ok(DecoderNotification::Error(err)) => self.publish_error(err),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }
    }

    fn drain_backend_notifications(&mut self) {
        loop {
            match self.backend_notify_rx.try_recv() {
                Ok(BackendNotification::StreamError { stream_id, reason }) => {
                    let Some(playback) = self.playback.as_ref() else {
                        continue;
                    };
                    if playback.stream_id != stream_id {
                        continue;
                    }

                    if self.state != EngineState::Playing {
                        warn!(
                            "ignore backend stream error while state={:?}: {}",
                            self.state, reason
                        );
                        continue;
                    }

                    let backend = playback.backend;
                    let device = Some(playback.device_name.clone());
                    self.event_hub
                        .publish(AudioEvent::DeviceLost { backend, device });
                    if let Err(err) = self.recover_from_device_loss(reason) {
                        self.publish_error(err);
                    }
                }
                Ok(BackendNotification::PlaybackDrained { stream_id }) => {
                    let Some(playback) = self.playback.as_ref() else {
                        continue;
                    };
                    if playback.stream_id != stream_id {
                        continue;
                    }
                    if self.state != EngineState::Playing {
                        continue;
                    }

                    self.position_base_ms = self.duration_ms;
                    self.playing_anchor = None;
                    self.event_hub.publish(AudioEvent::TrackEnded);
                    if let Err(err) = self.transition_to(EngineState::Ready) {
                        self.publish_error(err);
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }
    }

    fn recover_from_device_loss(&mut self, reason: String) -> Result<()> {
        self.transition_to(EngineState::Recovering)?;

        if self.current_source.is_none() {
            self.playing_anchor = None;
            self.transition_to(EngineState::Paused)?;
            return Err(AudioError::DeviceLost { reason });
        }

        self.config.preferred_device = None;
        match self.restart_current_source() {
            Ok(()) => Ok(()),
            Err(err) => {
                self.playing_anchor = None;
                self.transition_to(EngineState::Paused)?;
                Err(err)
            }
        }
    }

    fn transition_to(&mut self, next: EngineState) -> Result<()> {
        if self.state == next {
            return Ok(());
        }

        if !self.state.can_transition_to(next) {
            return Err(AudioError::InvalidStateTransition {
                from: self.state,
                to: next,
            });
        }

        let from = self.state;
        self.state = next;
        self.event_hub
            .publish(AudioEvent::StateChanged { from, to: next });
        Ok(())
    }

    fn publish_error(&mut self, err: AudioError) {
        warn!("audio runtime error: {err}");
        self.event_hub.publish(AudioEvent::Error(err.clone()));

        let position = self.current_position_ms();
        self.position_base_ms = position;
        if let Some(playback) = &self.playback {
            let _ = playback.output.pause();
        }

        let fallback_state = if self.current_source.is_some() {
            EngineState::Paused
        } else {
            EngineState::Error
        };
        let _ = self.transition_to(fallback_state);
        self.playing_anchor = None;
        self.publish_snapshot();
    }

    fn publish_snapshot(&self) {
        let snapshot = AudioSnapshot {
            state: self.state,
            is_playing: self.state == EngineState::Playing,
            position_ms: self.current_position_ms(),
            duration_ms: self.duration_ms,
            volume: self.config.volume,
            backend: self.config.backend,
            device: self
                .playback
                .as_ref()
                .map(|playback| playback.device_name.clone()),
            source: self.current_source.clone(),
        };

        if let Ok(mut latest) = self.latest_snapshot.write() {
            *latest = snapshot.clone();
        }

        self.snapshot_hub.publish(snapshot);
    }

    fn current_position_ms(&self) -> u64 {
        if let Some(anchor) = self.playing_anchor {
            let elapsed_ms = anchor.elapsed().as_millis() as u64;
            let position = self.position_base_ms.saturating_add(elapsed_ms);
            if self.duration_ms > 0 {
                return position.min(self.duration_ms);
            }
            return position;
        }

        if self.duration_ms > 0 {
            self.position_base_ms.min(self.duration_ms)
        } else {
            self.position_base_ms
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_interval_is_clamped() {
        let mut config = AudioConfig {
            snapshot_hz: 0,
            ..AudioConfig::default()
        };
        config.apply_patch(RuntimeConfigPatch {
            snapshot_hz: Some(0),
            ..RuntimeConfigPatch::default()
        });
        assert_eq!(config.snapshot_hz, 1);
    }
}
