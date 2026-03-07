use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread::JoinHandle;

use crate::command::AudioCommand;
use crate::config::AudioConfig;
use crate::error::{AudioError, Result};
use crate::event::AudioEvent;
use crate::runtime::AudioRuntime;
use crate::snapshot::AudioSnapshot;

#[derive(Clone)]
pub struct SubscriptionHub<T: Clone> {
    subscribers: Arc<Mutex<Vec<mpsc::Sender<T>>>>,
}

impl<T: Clone> Default for SubscriptionHub<T> {
    fn default() -> Self {
        Self {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl<T: Clone> SubscriptionHub<T> {
    pub fn subscribe(&self) -> mpsc::Receiver<T> {
        let (tx, rx) = mpsc::channel();
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(tx);
        }
        rx
    }

    pub fn publish(&self, value: T) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.retain(|subscriber| subscriber.send(value.clone()).is_ok());
        }
    }
}

#[derive(Clone)]
pub struct AudioService {
    command_tx: mpsc::Sender<AudioCommand>,
    event_hub: SubscriptionHub<AudioEvent>,
    snapshot_hub: SubscriptionHub<AudioSnapshot>,
    latest_snapshot: Arc<RwLock<AudioSnapshot>>,
}

impl AudioService {
    pub fn spawn(config: AudioConfig) -> Result<(Self, AudioRuntimeHandle)> {
        let (command_tx, command_rx) = mpsc::channel();
        let event_hub = SubscriptionHub::<AudioEvent>::default();
        let snapshot_hub = SubscriptionHub::<AudioSnapshot>::default();
        let latest_snapshot = Arc::new(RwLock::new(AudioSnapshot {
            volume: config.volume,
            backend: config.backend,
            ..AudioSnapshot::default()
        }));

        let runtime = AudioRuntime::new(
            config,
            command_rx,
            event_hub.clone(),
            snapshot_hub.clone(),
            Arc::clone(&latest_snapshot),
        )?;

        let thread = std::thread::Builder::new()
            .name("ame-audio-runtime".into())
            .spawn(move || runtime.run())
            .map_err(|err| AudioError::RuntimeJoinFailed {
                reason: err.to_string(),
            })?;

        let service = Self {
            command_tx: command_tx.clone(),
            event_hub,
            snapshot_hub,
            latest_snapshot,
        };

        let handle = AudioRuntimeHandle {
            command_tx,
            thread: Some(thread),
        };

        Ok((service, handle))
    }

    pub fn send(&self, command: AudioCommand) -> Result<()> {
        self.command_tx
            .send(command)
            .map_err(|_| AudioError::ChannelClosed)
    }

    pub fn subscribe_events(&self) -> mpsc::Receiver<AudioEvent> {
        self.event_hub.subscribe()
    }

    pub fn subscribe_snapshot(&self) -> mpsc::Receiver<AudioSnapshot> {
        self.snapshot_hub.subscribe()
    }

    pub fn snapshot(&self) -> AudioSnapshot {
        self.latest_snapshot
            .read()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default()
    }
}

pub struct AudioRuntimeHandle {
    command_tx: mpsc::Sender<AudioCommand>,
    thread: Option<JoinHandle<()>>,
}

impl AudioRuntimeHandle {
    pub fn shutdown(mut self) -> Result<()> {
        self.command_tx
            .send(AudioCommand::Shutdown)
            .map_err(|_| AudioError::ChannelClosed)?;

        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| AudioError::RuntimeJoinFailed {
                reason: "audio runtime panicked".into(),
            })?;
        }

        Ok(())
    }
}

impl Drop for AudioRuntimeHandle {
    fn drop(&mut self) {
        let _ = self.command_tx.send(AudioCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
