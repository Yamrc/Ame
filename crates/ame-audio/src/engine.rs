use std::time::Duration;

use cpal::{Device, StreamConfig};
use ringbuf::HeapRb;
use ringbuf::traits::Split;
use tracing::{debug, info};

use crate::Result;
use crate::decoder::{Decoder, Sample};
use crate::output::{OutputStream, default_config, default_device};
use crate::source::Source;

pub struct AudioEngine {
    device: Device,
    config: StreamConfig,
    sample_format: cpal::SampleFormat,
    output: Option<OutputStream>,
    decoder_handle: Option<std::thread::JoinHandle<Result<()>>>,
}

impl AudioEngine {
    pub fn new() -> Result<Self> {
        let device = default_device().ok_or(crate::AudioError::DeviceNotAvailable)?;
        let (config, sample_format) = default_config(&device)?;

        info!(
            "AudioEngine initialized: sample_rate={}, channels={}, format={:?}",
            config.sample_rate, config.channels, sample_format
        );

        Ok(Self {
            device,
            config,
            sample_format,
            output: None,
            decoder_handle: None,
        })
    }

    pub fn with_device(device: Device) -> Result<Self> {
        let (config, sample_format) = default_config(&device)?;

        Ok(Self {
            device,
            config,
            sample_format,
            output: None,
            decoder_handle: None,
        })
    }

    pub fn play(&mut self, source: Box<dyn Source>) -> Result<()> {
        self.stop();

        let sample_rate = self.config.sample_rate;
        let channels = self.config.channels as usize;

        info!(
            "Starting playback: sample_rate={}, channels={}",
            sample_rate, channels
        );

        let ring_capacity = sample_rate as usize * channels * 2;
        debug!("Ring buffer capacity: {} samples", ring_capacity);

        let rb = HeapRb::<Sample>::new(ring_capacity);
        let (prod, cons) = rb.split();

        let media_source = source.into_media_source();
        self.decoder_handle = Some(Decoder::spawn(media_source, sample_rate, prod));

        let output = OutputStream::new(&self.device, &self.config, self.sample_format, cons)?;
        output.play()?;
        self.output = Some(output);

        info!("Playback started");

        Ok(())
    }

    pub fn pause(&self) {
        if let Some(ref output) = self.output {
            let _ = output.pause();
        }
    }

    pub fn resume(&self) {
        if let Some(ref output) = self.output {
            let _ = output.play();
        }
    }

    pub fn stop(&mut self) {
        self.output = None;
        self.decoder_handle = None;
    }

    pub fn set_volume(&self, volume: f32) {
        let clamped = volume.clamp(0.0, 1.0);
        debug!("Volume set to: {:.2}", clamped);
        if let Some(ref output) = self.output {
            output.set_volume(clamped);
        }
    }

    pub fn set_device(&mut self, device: Device) -> Result<()> {
        let (config, sample_format) = default_config(&device)?;
        self.device = device;
        self.config = config;
        self.sample_format = sample_format;

        self.stop();

        Ok(())
    }

    pub fn current_position(&self) -> Duration {
        Duration::ZERO
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new().expect("default audio engine")
    }
}
