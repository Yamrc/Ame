use std::sync::mpsc::Receiver;

use ame_audio::{AudioCommand, AudioEvent, AudioService, AudioSnapshot};

use crate::domain::player::PlayerEntity;

pub struct AudioBridgeEntity {
    service: AudioService,
    event_rx: Receiver<AudioEvent>,
    snapshot_rx: Receiver<AudioSnapshot>,
    pub last_error: Option<String>,
}

impl AudioBridgeEntity {
    pub fn new(service: AudioService) -> Self {
        let event_rx = service.subscribe_events();
        let snapshot_rx = service.subscribe_snapshot();
        Self {
            service,
            event_rx,
            snapshot_rx,
            last_error: None,
        }
    }

    pub fn service(&self) -> AudioService {
        self.service.clone()
    }

    pub fn send(&self, command: AudioCommand) -> ame_audio::Result<()> {
        self.service.send(command)
    }

    pub fn drain(&mut self, player: &mut PlayerEntity) -> Vec<AudioEvent> {
        while let Ok(snapshot) = self.snapshot_rx.try_recv() {
            let is_idle_snapshot = snapshot.source.is_none()
                && !snapshot.is_playing
                && snapshot.position_ms == 0
                && snapshot.duration_ms == 0;
            if is_idle_snapshot {
                continue;
            }
            player.apply_audio_snapshot(&snapshot);
        }

        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            if let AudioEvent::Error(err) = &event {
                self.last_error = Some(err.to_string());
            }
            events.push(event);
        }
        events
    }
}
