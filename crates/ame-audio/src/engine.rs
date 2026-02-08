use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cpal::{Device, StreamConfig};
use ringbuf::traits::Split;
use ringbuf::HeapRb;
use tracing::{debug, info, warn};

use crate::decoder::{Decoder, Sample};
use crate::output::{default_config, default_device, OutputStream};
use crate::source::{FileSource, Source};
use crate::Result;

pub struct AudioEngine {
    device: Device,
    config: StreamConfig,
    sample_format: cpal::SampleFormat,
    output: Option<OutputStream>,
    decoder_handle: Option<std::thread::JoinHandle<Result<()>>>,
    current_file: Option<FileSource>,
    position_tracker: Arc<AtomicU64>,
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
            current_file: None,
            position_tracker: Arc::new(AtomicU64::new(0)),
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
            current_file: None,
            position_tracker: Arc::new(AtomicU64::new(0)),
        })
    }

    pub fn play_file(&mut self, source: FileSource) -> Result<()> {
        self.stop();

        // Store the file source for seeking
        self.current_file = Some(FileSource::new(source.path())?);

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

        // Reset position tracker
        self.position_tracker.store(0, Ordering::Relaxed);

        let media_source = Box::new(source).into_media_source();
        self.decoder_handle = Some(Decoder::spawn(media_source, sample_rate, prod));

        let output = OutputStream::new(&self.device, &self.config, self.sample_format, cons)?;
        output.play()?;
        self.output = Some(output);

        info!("Playback started");

        Ok(())
    }

    pub fn play(&mut self, source: Box<dyn Source>) -> Result<()> {
        // For generic Source, we can't seek (no way to recreate it)
        // Use play_file() for seekable file playback
        self.stop();

        let sample_rate = self.config.sample_rate;
        let channels = self.config.channels as usize;

        info!(
            "Starting playback (non-seekable): sample_rate={}, channels={}",
            sample_rate, channels
        );

        let ring_capacity = sample_rate as usize * channels * 2;
        debug!("Ring buffer capacity: {} samples", ring_capacity);

        let rb = HeapRb::<Sample>::new(ring_capacity);
        let (prod, cons) = rb.split();

        // Reset position tracker
        self.position_tracker.store(0, Ordering::Relaxed);

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
        self.current_file = None;
        self.position_tracker.store(0, Ordering::Relaxed);
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
        let ms = self.position_tracker.load(Ordering::Relaxed);
        Duration::from_millis(ms)
    }

    pub fn seek_to(&mut self, position: Duration) -> Result<()> {
        // Check if we have an active file stored
        let file_source = match &self.current_file {
            Some(fs) => fs.reopen()?,
            None => {
                warn!("No file source available for seek");
                return Err(crate::AudioError::DeviceNotAvailable);
            }
        };

        let is_playing = self.output.is_some();

        // Stop current playback
        self.output = None;
        self.decoder_handle = None;

        let sample_rate = self.config.sample_rate;
        let channels = self.config.channels as usize;

        // Update position tracker immediately
        self.position_tracker
            .store(position.as_millis() as u64, Ordering::Relaxed);

        let ring_capacity = sample_rate as usize * channels * 2;
        let rb = HeapRb::<Sample>::new(ring_capacity);
        let (prod, cons) = rb.split();

        // Use spawn_at to start from the specified position
        let media_source = Box::new(file_source).into_media_source();
        self.decoder_handle = Some(Decoder::spawn_at(
            media_source,
            sample_rate,
            prod,
            position,
            Some(Arc::clone(&self.position_tracker)),
        ));

        if is_playing {
            let output = OutputStream::new(&self.device, &self.config, self.sample_format, cons)?;
            output.play()?;
            self.output = Some(output);
            info!("Seek completed and resumed playback at {:?}", position);
        } else {
            info!("Seek completed, ready to play from {:?}", position);
        }

        Ok(())
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new().expect("default audio engine")
    }
}
