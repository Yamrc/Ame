use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;

use audioadapter_buffers::direct::InterleavedSlice;
use ringbuf::traits::Producer;
use rubato::{
    Async, FixedAsync, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;
use symphonia::default::{get_codecs, get_probe};
use tracing::debug;

use crate::Sample;
use crate::backend::SampleRing;
use crate::config::ResampleQualityPreset;
use crate::error::{AudioError, Result};
use crate::source::{SourceFactory, SourceSpec};

pub(crate) type RingProducer = <SampleRing as ringbuf::traits::Split>::Prod;

#[derive(Debug, Clone)]
pub(crate) enum DecoderNotification {
    Duration(u64),
    Ended,
    Error(AudioError),
}

pub(crate) struct DecoderSpawnRequest {
    pub source_factory: Arc<dyn SourceFactory>,
    pub source_spec: SourceSpec,
    pub target_sample_rate: u32,
    pub target_channels: usize,
    pub start_ms: u64,
    pub quality: ResampleQualityPreset,
    pub cancel: Arc<AtomicBool>,
    pub notification_tx: Sender<DecoderNotification>,
    pub producer: RingProducer,
}

pub(crate) fn spawn_decoder(request: DecoderSpawnRequest) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let DecoderSpawnRequest {
            source_factory,
            source_spec,
            target_sample_rate,
            target_channels,
            start_ms,
            quality,
            cancel,
            notification_tx,
            producer,
        } = request;
        let result = decode_loop(
            source_factory,
            source_spec,
            target_sample_rate,
            target_channels,
            start_ms,
            quality,
            cancel,
            notification_tx.clone(),
            producer,
        );

        if let Err(err) = result {
            let _ = notification_tx.send(DecoderNotification::Error(err));
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn decode_loop(
    source_factory: Arc<dyn SourceFactory>,
    source_spec: SourceSpec,
    target_sample_rate: u32,
    target_channels: usize,
    start_ms: u64,
    quality: ResampleQualityPreset,
    cancel: Arc<AtomicBool>,
    notification_tx: Sender<DecoderNotification>,
    mut producer: RingProducer,
) -> Result<()> {
    let opened = source_factory.open(&source_spec)?;
    if start_ms > 0 && !opened.seekable {
        return Err(AudioError::UnsupportedSeek);
    }

    let media_stream = MediaSourceStream::new(opened.media_source, Default::default());
    let probed = get_probe()
        .format(
            &Default::default(),
            media_stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|err| AudioError::DecodeFailed {
            reason: err.to_string(),
        })?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| AudioError::DecodeFailed {
            reason: "missing default track".into(),
        })?;

    let track_id = track.id;
    let codec_params = &track.codec_params;
    let src_sample_rate = codec_params
        .sample_rate
        .ok_or_else(|| AudioError::DecodeFailed {
            reason: "unknown source sample rate".into(),
        })? as u32;
    let src_channels = codec_params
        .channels
        .ok_or_else(|| AudioError::DecodeFailed {
            reason: "unknown channel layout".into(),
        })?
        .count();

    if let (Some(frames), Some(sample_rate)) = (codec_params.n_frames, codec_params.sample_rate) {
        let duration_ms = frames.saturating_mul(1000) / sample_rate as u64;
        let _ = notification_tx.send(DecoderNotification::Duration(duration_ms));
    }

    let mut decoder = get_codecs()
        .make(codec_params, &DecoderOptions::default())
        .map_err(|err| AudioError::DecodeFailed {
            reason: err.to_string(),
        })?;

    if start_ms > 0 {
        let seconds = start_ms / 1000;
        let frac = (start_ms % 1000) as f64 / 1000.0;
        format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::new(seconds, frac),
                    track_id: Some(track_id),
                },
            )
            .map_err(|err| AudioError::DecodeFailed {
                reason: format!("seek failed: {err}"),
            })?;
    }

    let mut resampler = if src_sample_rate != target_sample_rate {
        Some(ResamplerPipeline::new(
            src_sample_rate,
            target_sample_rate,
            src_channels,
            quality,
        )?)
    } else {
        None
    };

    let mut channel_scratch = Vec::new();

    loop {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(io_err))
                if io_err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::IoError(io_err))
                if io_err.kind() == std::io::ErrorKind::PermissionDenied =>
            {
                return Err(AudioError::HttpStatus {
                    code: 403,
                    url: source_spec.describe(),
                });
            }
            Err(err) => {
                return Err(AudioError::DecodeFailed {
                    reason: err.to_string(),
                });
            }
        };

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(err) => {
                return Err(AudioError::DecodeFailed {
                    reason: err.to_string(),
                });
            }
        };

        let mut sample_buf = SampleBuffer::<Sample>::new(decoded.frames() as u64, *decoded.spec());
        sample_buf.copy_interleaved_ref(decoded);

        let interleaved = sample_buf.samples();
        let output_samples = if let Some(resampler) = &mut resampler {
            resampler.process(interleaved, &mut producer, &cancel)?;
            None
        } else {
            Some(interleaved)
        };

        if let Some(samples) = output_samples {
            let channels_adjusted =
                convert_channels(samples, src_channels, target_channels, &mut channel_scratch);
            push_samples(&mut producer, channels_adjusted, &cancel)?;
        }
    }

    debug!("decoder finished for {}", source_spec.describe());
    if !cancel.load(Ordering::Relaxed) {
        let _ = notification_tx.send(DecoderNotification::Ended);
    }

    Ok(())
}

fn convert_channels<'a>(
    samples: &'a [Sample],
    input_channels: usize,
    output_channels: usize,
    scratch: &'a mut Vec<Sample>,
) -> &'a [Sample] {
    if input_channels == output_channels || input_channels == 0 || output_channels == 0 {
        return samples;
    }

    scratch.clear();
    let frames = samples.len() / input_channels;
    scratch.reserve(frames * output_channels);

    match (input_channels, output_channels) {
        (1, 2) => {
            for &sample in samples {
                scratch.push(sample);
                scratch.push(sample);
            }
        }
        (2, 1) => {
            for frame in samples.chunks_exact(2) {
                scratch.push((frame[0] + frame[1]) * 0.5);
            }
        }
        _ => {
            for frame in samples.chunks_exact(input_channels) {
                for channel in 0..output_channels {
                    scratch.push(*frame.get(channel).unwrap_or(&0.0));
                }
            }
        }
    }

    scratch.as_slice()
}

fn push_samples(
    producer: &mut RingProducer,
    samples: &[Sample],
    cancel: &Arc<AtomicBool>,
) -> Result<()> {
    let mut offset = 0;
    while offset < samples.len() {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let pushed = producer.push_slice(&samples[offset..]);
        if pushed == 0 {
            thread::sleep(std::time::Duration::from_micros(250));
            continue;
        }
        offset += pushed;
    }
    Ok(())
}

struct ResamplerPipeline {
    resampler: Box<dyn rubato::Resampler<Sample>>,
    channels: usize,
    input: Vec<Sample>,
    input_start: usize,
    output: Vec<Sample>,
}

impl ResamplerPipeline {
    fn new(
        src_sample_rate: u32,
        target_sample_rate: u32,
        channels: usize,
        quality: ResampleQualityPreset,
    ) -> Result<Self> {
        let ratio = target_sample_rate as f64 / src_sample_rate as f64;
        let (sinc_len, cutoff, oversampling) = match quality {
            ResampleQualityPreset::LowLatency => (32, 0.90, 64),
            ResampleQualityPreset::Balanced | ResampleQualityPreset::Auto => (64, 0.88, 128),
            ResampleQualityPreset::HighQuality => (128, 0.92, 256),
        };

        let params = SincInterpolationParameters {
            sinc_len,
            f_cutoff: cutoff,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: oversampling,
            window: WindowFunction::BlackmanHarris2,
        };

        let resampler =
            Async::<Sample>::new_sinc(ratio, 1.0, &params, 1024, channels, FixedAsync::Input)
                .map_err(|err| AudioError::ConfigInvalid {
                    reason: err.to_string(),
                })?;

        Ok(Self {
            resampler: Box::new(resampler),
            channels,
            input: Vec::with_capacity(16 * 1024),
            input_start: 0,
            output: Vec::with_capacity(16 * 1024),
        })
    }

    fn process(
        &mut self,
        samples: &[Sample],
        producer: &mut RingProducer,
        cancel: &Arc<AtomicBool>,
    ) -> Result<()> {
        self.input.extend_from_slice(samples);

        let needed_frames = self.resampler.input_frames_next();

        while (self.input.len() - self.input_start) / self.channels >= needed_frames {
            let in_elems = needed_frames * self.channels;
            let start = self.input_start;
            let end = start + in_elems;

            let input =
                InterleavedSlice::new(&self.input[start..end], self.channels, needed_frames)
                    .map_err(|err| AudioError::DecodeFailed {
                        reason: err.to_string(),
                    })?;

            let out_frames = self.resampler.process_all_needed_output_len(needed_frames);
            let out_len = out_frames * self.channels;
            if self.output.len() < out_len {
                self.output.resize(out_len, 0.0);
            }

            let mut output =
                InterleavedSlice::new_mut(&mut self.output[..out_len], self.channels, out_frames)
                    .map_err(|err| AudioError::DecodeFailed {
                    reason: err.to_string(),
                })?;

            let (consumed_frames, produced_frames) = self
                .resampler
                .process_into_buffer(&input, &mut output, None)
                .map_err(|err| AudioError::DecodeFailed {
                    reason: err.to_string(),
                })?;

            let produced_samples = produced_frames * self.channels;
            push_samples(producer, &self.output[..produced_samples], cancel)?;

            self.input_start += consumed_frames * self.channels;
            if self.input_start > self.input.len() / 2 {
                let remaining = self.input.len() - self.input_start;
                self.input.copy_within(self.input_start.., 0);
                self.input.truncate(remaining);
                self.input_start = 0;
            }

            if cancel.load(Ordering::Relaxed) {
                break;
            }
        }

        Ok(())
    }
}
