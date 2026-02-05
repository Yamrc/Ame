use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use ringbuf::traits::{Consumer, Split};
use tracing::{debug, error, info};

use crate::Result;
use crate::decoder::{RingBuf, Sample};

pub struct OutputStream {
    stream: Stream,
    volume: Arc<AtomicU32>,
}

impl OutputStream {
    pub fn new(
        device: &Device,
        config: &StreamConfig,
        sample_format: SampleFormat,
        consumer: <RingBuf as Split>::Cons,
    ) -> Result<Self> {
        let volume = Arc::new(AtomicU32::new(1.0f32.to_bits()));
        let vol_clone = volume.clone();

        debug!("Building output stream with format: {:?}", sample_format);

        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(device, config, consumer, vol_clone)?,
            SampleFormat::I16 => build_stream::<i16>(device, config, consumer, vol_clone)?,
            SampleFormat::U16 => build_stream::<u16>(device, config, consumer, vol_clone)?,
            _ => return Err(crate::AudioError::UnsupportedFormat),
        };

        info!("Output stream created successfully");
        Ok(Self { stream, volume })
    }

    pub fn play(&self) -> Result<()> {
        debug!("Output stream playing");
        self.stream.play()?;
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        debug!("Output stream paused");
        self.stream.pause()?;
        Ok(())
    }

    pub fn set_volume(&self, volume: f32) {
        self.volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}

fn build_stream<T: cpal::SizedSample + cpal::FromSample<Sample>>(
    device: &Device,
    config: &StreamConfig,
    mut consumer: <RingBuf as Split>::Cons,
    volume: Arc<AtomicU32>,
) -> Result<Stream> {
    let err_fn = |err: cpal::StreamError| eprintln!("CPAL error: {:?}", err);

    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let vol = f32::from_bits(volume.load(Ordering::Relaxed));
                let mut f32_buf = vec![0.0; data.len()];
                consumer.pop_slice(&mut f32_buf);

                for (out, &sample) in data.iter_mut().zip(&f32_buf) {
                    *out = T::from_sample(sample * vol);
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| e.into())
}

pub fn default_device() -> Option<Device> {
    let device = cpal::default_host().default_output_device();
    if let Some(ref d) = device {
        debug!("Default output device: {:?}", d.id());
    } else {
        error!("No default output device found");
    }
    device
}

pub fn default_config(device: &Device) -> Result<(StreamConfig, SampleFormat)> {
    let supported = device.default_output_config()?;
    let format = supported.sample_format();
    Ok((supported.into(), format))
}
