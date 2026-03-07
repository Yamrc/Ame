use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
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
}

#[derive(Debug, Clone)]
pub enum BackendNotification {
    StreamError { stream_id: u64, reason: String },
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

        let volume = Arc::new(AtomicU32::new(request.volume.clamp(0.0, 1.0).to_bits()));
        let volume_clone = Arc::clone(&volume);
        let tx = request.event_tx;
        let stream_id = request.stream_id;

        let stream = match sample_format {
            SampleFormat::F32 => build_output_stream::<f32>(
                &device,
                &config,
                request.consumer,
                volume_clone,
                tx,
                stream_id,
            )?,
            SampleFormat::I16 => build_output_stream::<i16>(
                &device,
                &config,
                request.consumer,
                volume_clone,
                tx,
                stream_id,
            )?,
            SampleFormat::U16 => build_output_stream::<u16>(
                &device,
                &config,
                request.consumer,
                volume_clone,
                tx,
                stream_id,
            )?,
            _ => {
                return Err(AudioError::OutputInitFailed {
                    reason: format!("unsupported output sample format: {sample_format:?}"),
                });
            }
        };

        Ok(Box::new(CpalOutputSession {
            stream,
            volume,
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
    volume: Arc<AtomicU32>,
    event_tx: Sender<BackendNotification>,
    stream_id: u64,
) -> Result<Stream>
where
    T: cpal::SizedSample + cpal::FromSample<Sample>,
{
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            let volume = f32::from_bits(volume.load(Ordering::Relaxed));
            for output in data.iter_mut() {
                let sample = consumer.try_pop().unwrap_or_default();
                *output = T::from_sample(sample * volume);
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

struct CpalOutputSession {
    stream: Stream,
    volume: Arc<AtomicU32>,
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
        self.volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
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
