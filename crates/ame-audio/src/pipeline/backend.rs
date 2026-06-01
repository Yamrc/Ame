use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;

#[cfg(target_os = "windows")]
use cpal::HostId;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Split};

use crate::error::{AudioError, Result};
use crate::{OutputBackendKind, Sample};

pub type SampleRing = ringbuf::HeapRb<Sample>;
pub type RingConsumer = <SampleRing as Split>::Cons;

#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
}

pub struct OpenStreamRequest {
    pub stream_id: u64,
    pub preferred_device: Option<String>,
    pub volume: f32,
    pub consumer: RingConsumer,
    pub event_tx: Sender<BackendNotification>,
    pub drain_state: Arc<PlaybackDrainState>,
}

#[derive(Debug, Clone)]
pub enum BackendNotification {
    StreamError { stream_id: u64, reason: String },
    PlaybackDrained { stream_id: u64 },
}

#[derive(Debug, Default)]
pub struct PlaybackDrainState {
    pending_samples: AtomicUsize,
    decoder_finished: AtomicBool,
    drained_notified: AtomicBool,
}

impl PlaybackDrainState {
    pub fn on_samples_queued(&self, count: usize) {
        if count == 0 {
            return;
        }

        self.pending_samples.fetch_add(count, Ordering::AcqRel);
        self.drained_notified.store(false, Ordering::Release);
    }

    pub fn mark_decoder_finished(&self) {
        self.decoder_finished.store(true, Ordering::Release);
    }

    pub fn on_output_callback(&self, played_samples: usize) -> bool {
        if played_samples > 0 {
            self.consume_samples(played_samples);
            return false;
        }

        if !self.decoder_finished.load(Ordering::Acquire) {
            return false;
        }

        if self.pending_samples.load(Ordering::Acquire) > 0 {
            return false;
        }

        !self.drained_notified.swap(true, Ordering::AcqRel)
    }

    fn consume_samples(&self, count: usize) {
        let mut current = self.pending_samples.load(Ordering::Acquire);
        loop {
            let next = current.saturating_sub(count);
            match self.pending_samples.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }
}

pub trait OutputSession: Send {
    fn play(&self) -> Result<()>;
    fn pause(&self) -> Result<()>;
    fn set_volume(&self, volume: f32);
    fn sample_rate(&self) -> u32;
    fn channels(&self) -> u16;
    fn device_name(&self) -> String;
}

pub trait OutputBackend: Send + Sync {
    fn kind(&self) -> OutputBackendKind;
    fn list_devices(&self) -> Result<Vec<AudioDevice>>;
    fn default_device(&self) -> Result<AudioDevice>;
    fn open_stream(&self, request: OpenStreamRequest) -> Result<Box<dyn OutputSession>>;
}

pub struct CpalBackend {
    kind: OutputBackendKind,
}

impl CpalBackend {
    pub fn new(kind: OutputBackendKind) -> Self {
        Self { kind }
    }

    fn host(&self) -> Result<cpal::Host> {
        host_for_backend(self.kind)
    }

    fn pick_device(&self, preferred: Option<&str>) -> Result<Device> {
        let host = self.host()?;

        if let Some(preferred_name) = preferred {
            let mut devices =
                host.output_devices()
                    .map_err(|err| AudioError::OutputInitFailed {
                        reason: err.to_string(),
                    })?;

            if let Some(device) =
                devices.find(|d| device_name(d).is_ok_and(|name| name == preferred_name))
            {
                return Ok(device);
            }

            return Err(AudioError::DeviceNotAvailable {
                device: preferred_name.to_string(),
            });
        }

        host.default_output_device()
            .ok_or_else(|| AudioError::DeviceNotAvailable {
                device: "default".into(),
            })
    }
}

impl OutputBackend for CpalBackend {
    fn kind(&self) -> OutputBackendKind {
        self.kind
    }

    fn list_devices(&self) -> Result<Vec<AudioDevice>> {
        let host = self.host()?;
        let devices = host
            .output_devices()
            .map_err(|err| AudioError::OutputInitFailed {
                reason: err.to_string(),
            })?;

        Ok(devices
            .filter_map(|device| {
                let name = device_name(&device).ok()?;
                Some(AudioDevice {
                    id: name.clone(),
                    name,
                })
            })
            .collect())
    }

    fn default_device(&self) -> Result<AudioDevice> {
        let host = self.host()?;
        let device =
            host.default_output_device()
                .ok_or_else(|| AudioError::DeviceNotAvailable {
                    device: "default".into(),
                })?;

        let name = device_name(&device).map_err(|err| AudioError::OutputInitFailed {
            reason: err.to_string(),
        })?;

        Ok(AudioDevice {
            id: name.clone(),
            name,
        })
    }

    fn open_stream(&self, request: OpenStreamRequest) -> Result<Box<dyn OutputSession>> {
        let device = self.pick_device(request.preferred_device.as_deref())?;
        let device_name = device_name(&device).map_err(|err| AudioError::OutputInitFailed {
            reason: err.to_string(),
        })?;

        let supported = device.default_output_config()?;
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();

        let gain = Arc::new(AtomicU32::new(volume_to_gain(request.volume).to_bits()));
        let gain_clone = Arc::clone(&gain);
        let tx = request.event_tx;
        let stream_id = request.stream_id;
        let drain_state = request.drain_state;

        let stream = match sample_format {
            SampleFormat::F32 => build_output_stream::<f32>(
                &device,
                &config,
                request.consumer,
                gain_clone,
                tx,
                stream_id,
                Arc::clone(&drain_state),
            )?,
            SampleFormat::I16 => build_output_stream::<i16>(
                &device,
                &config,
                request.consumer,
                gain_clone,
                tx,
                stream_id,
                Arc::clone(&drain_state),
            )?,
            SampleFormat::U16 => build_output_stream::<u16>(
                &device,
                &config,
                request.consumer,
                gain_clone,
                tx,
                stream_id,
                drain_state,
            )?,
            _ => {
                return Err(AudioError::OutputInitFailed {
                    reason: format!("unsupported output sample format: {sample_format:?}"),
                });
            }
        };

        Ok(Box::new(CpalOutputSession {
            stream,
            gain,
            sample_rate: config.sample_rate,
            channels: config.channels,
            device_name,
        }))
    }
}

fn build_output_stream<T>(
    device: &Device,
    config: &StreamConfig,
    mut consumer: RingConsumer,
    gain: Arc<AtomicU32>,
    event_tx: Sender<BackendNotification>,
    stream_id: u64,
    drain_state: Arc<PlaybackDrainState>,
) -> Result<Stream>
where
    T: cpal::SizedSample + cpal::FromSample<Sample>,
{
    let drain_event_tx = event_tx.clone();
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            let gain = f32::from_bits(gain.load(Ordering::Relaxed));
            let mut played_samples = 0;
            for output in data.iter_mut() {
                if let Some(sample) = consumer.try_pop() {
                    played_samples += 1;
                    *output = T::from_sample(sample * gain);
                } else {
                    *output = T::from_sample(0.0);
                }
            }

            if drain_state.on_output_callback(played_samples) {
                let _ = drain_event_tx.send(BackendNotification::PlaybackDrained { stream_id });
            }
        },
        move |err| {
            let _ = event_tx.send(BackendNotification::StreamError {
                stream_id,
                reason: err.to_string(),
            });
        },
        None,
    )?;

    Ok(stream)
}

const VOLUME_GAIN_RANGE_DB: f32 = 60.0;

fn volume_to_gain(volume: f32) -> f32 {
    let volume = volume.clamp(0.0, 1.0);
    if volume <= 0.0 {
        0.0
    } else {
        10.0_f32.powf((-VOLUME_GAIN_RANGE_DB * (1.0 - volume)) / 20.0)
    }
}

struct CpalOutputSession {
    stream: Stream,
    gain: Arc<AtomicU32>,
    sample_rate: u32,
    channels: u16,
    device_name: String,
}

impl OutputSession for CpalOutputSession {
    fn play(&self) -> Result<()> {
        self.stream.play()?;
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        self.stream.pause()?;
        Ok(())
    }

    fn set_volume(&self, volume: f32) {
        self.gain
            .store(volume_to_gain(volume).to_bits(), Ordering::Relaxed);
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn device_name(&self) -> String {
        self.device_name.clone()
    }
}

pub fn backend_for_kind(kind: OutputBackendKind) -> Result<Box<dyn OutputBackend>> {
    match kind {
        OutputBackendKind::PlatformDefault => Ok(Box::new(CpalBackend::new(kind))),
        OutputBackendKind::Wasapi => {
            #[cfg(target_os = "windows")]
            {
                Ok(Box::new(CpalBackend::new(kind)))
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err(AudioError::BackendUnavailable { backend: kind })
            }
        }
        OutputBackendKind::Asio => {
            #[cfg(all(target_os = "windows", feature = "backend-asio"))]
            {
                Ok(Box::new(CpalBackend::new(kind)))
            }
            #[cfg(any(not(target_os = "windows"), not(feature = "backend-asio")))]
            {
                Err(AudioError::BackendUnavailable { backend: kind })
            }
        }
    }
}

fn host_for_backend(kind: OutputBackendKind) -> Result<cpal::Host> {
    match kind {
        OutputBackendKind::PlatformDefault => Ok(cpal::default_host()),
        OutputBackendKind::Wasapi => {
            #[cfg(target_os = "windows")]
            {
                cpal::host_from_id(HostId::Wasapi).map_err(|err| AudioError::OutputInitFailed {
                    reason: err.to_string(),
                })
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err(AudioError::BackendUnavailable { backend: kind })
            }
        }
        OutputBackendKind::Asio => {
            #[cfg(all(target_os = "windows", feature = "backend-asio"))]
            {
                cpal::host_from_id(HostId::Asio).map_err(|err| AudioError::OutputInitFailed {
                    reason: err.to_string(),
                })
            }
            #[cfg(any(not(target_os = "windows"), not(feature = "backend-asio")))]
            {
                Err(AudioError::BackendUnavailable { backend: kind })
            }
        }
    }
}

#[allow(deprecated)]
fn device_name(device: &Device) -> std::result::Result<String, cpal::DeviceNameError> {
    device.name()
}

#[cfg(test)]
mod tests {
    use super::{PlaybackDrainState, volume_to_gain};

    fn assert_approx_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.000_001,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn volume_to_gain_preserves_mute_and_unity() {
        assert_eq!(volume_to_gain(-1.0), 0.0);
        assert_eq!(volume_to_gain(0.0), 0.0);
        assert_eq!(volume_to_gain(1.0), 1.0);
        assert_eq!(volume_to_gain(2.0), 1.0);
    }

    #[test]
    fn volume_to_gain_uses_sixty_db_curve() {
        assert_approx_eq(volume_to_gain(0.01), 10.0_f32.powf(-59.4 / 20.0));
        assert_approx_eq(volume_to_gain(0.5), 10.0_f32.powf(-30.0 / 20.0));
        assert_approx_eq(volume_to_gain(0.7), 10.0_f32.powf(-18.0 / 20.0));
    }

    #[test]
    fn volume_to_gain_is_monotonic_and_quiet_values_are_below_linear_gain() {
        assert!(volume_to_gain(0.01) < 0.01);
        assert!(volume_to_gain(0.1) < volume_to_gain(0.5));
        assert!(volume_to_gain(0.5) < volume_to_gain(1.0));
    }

    #[test]
    fn drain_notification_waits_for_empty_callback_after_last_samples() {
        let state = PlaybackDrainState::default();
        state.on_samples_queued(8);

        assert!(!state.on_output_callback(0));

        state.mark_decoder_finished();

        assert!(!state.on_output_callback(4));
        assert!(!state.on_output_callback(4));
        assert!(state.on_output_callback(0));
    }

    #[test]
    fn drain_notification_is_emitted_only_once() {
        let state = PlaybackDrainState::default();
        state.mark_decoder_finished();

        assert!(state.on_output_callback(0));
        assert!(!state.on_output_callback(0));
    }
}
