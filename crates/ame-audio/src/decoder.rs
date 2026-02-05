use ringbuf::HeapRb;
use ringbuf::traits::Producer;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::{get_codecs, get_probe};
use tracing::{debug, info};

use crate::Result;

pub type Sample = f32;
pub type RingBuf = HeapRb<Sample>;

pub struct Decoder;

impl Decoder {
    pub fn spawn(
        source: Box<dyn symphonia::core::io::MediaSource>,
        output_sample_rate: u32,
        ring_prod: <RingBuf as ringbuf::traits::Split>::Prod,
    ) -> std::thread::JoinHandle<Result<()>> {
        std::thread::spawn(move || Self::decode_loop(source, output_sample_rate, ring_prod))
    }

    fn decode_loop(
        source: Box<dyn symphonia::core::io::MediaSource>,
        output_sample_rate: u32,
        mut prod: <RingBuf as ringbuf::traits::Split>::Prod,
    ) -> Result<()> {
        info!(
            "Decoder started, output sample rate: {} Hz",
            output_sample_rate
        );

        let mss = MediaSourceStream::new(source, Default::default());
        let probed = get_probe()
            .format(
                &Default::default(),
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| crate::AudioError::Decode(e.to_string()))?;

        let mut format = probed.format;
        let track = format
            .default_track()
            .ok_or(crate::AudioError::UnsupportedFormat)?;
        let codec_params = &track.codec_params;

        debug!(
            "Codec: {:?}, Sample Rate: {:?}, Channels: {:?}",
            codec_params.codec, codec_params.sample_rate, codec_params.channels
        );

        let mut decoder = get_codecs()
            .make(codec_params, &DecoderOptions::default())
            .map_err(|e| crate::AudioError::Decode(e.to_string()))?;

        let src_rate = codec_params
            .sample_rate
            .ok_or(crate::AudioError::UnsupportedFormat)? as u32;
        let channels = codec_params
            .channels
            .ok_or(crate::AudioError::UnsupportedFormat)?
            .count();

        let need_resample = src_rate != output_sample_rate;
        info!(
            "Audio format: {} Hz, {} channels, resample needed: {}",
            src_rate, channels, need_resample
        );

        let mut resampler =
            need_resample.then(|| create_resampler(src_rate, output_sample_rate, channels));

        let mut in_buf: Vec<Sample> = Vec::with_capacity(8192);

        loop {
            use symphonia::core::errors::Error as SymphError;
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(SymphError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(crate::AudioError::Decode(e.to_string())),
            };

            if let Ok(audio_buf) = decoder.decode(&packet) {
                let mut sample_buf =
                    SampleBuffer::<Sample>::new(audio_buf.frames() as u64, *audio_buf.spec());
                sample_buf.copy_interleaved_ref(audio_buf);
                in_buf.extend_from_slice(sample_buf.samples());

                if let Some(ref mut r) = resampler {
                    process_resampling(r.as_mut(), &mut in_buf, channels, &mut prod)?;
                } else {
                    push_samples(&mut prod, &in_buf);
                    in_buf.clear();
                }
            }
        }

        info!("Decoding complete");
        Ok(())
    }
}

fn create_resampler(
    src_rate: u32,
    dst_rate: u32,
    channels: usize,
) -> Box<dyn rubato::Resampler<Sample>> {
    use rubato::{
        Async, FixedAsync, SincInterpolationParameters, SincInterpolationType, WindowFunction,
    };

    let ratio = dst_rate as f64 / src_rate as f64;
    info!(
        "Creating resampler: {} Hz -> {} Hz (ratio: {:.4})",
        src_rate, dst_rate, ratio
    );

    let params = SincInterpolationParameters {
        sinc_len: 64,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 128,
        window: WindowFunction::BlackmanHarris2,
    };

    Box::new(
        Async::<Sample>::new_sinc(ratio, 1.1, &params, 1024, channels, FixedAsync::Input)
            .expect("valid resampler params"),
    )
}

fn process_resampling(
    resampler: &mut dyn rubato::Resampler<Sample>,
    in_buf: &mut Vec<Sample>,
    channels: usize,
    prod: &mut <RingBuf as ringbuf::traits::Split>::Prod,
) -> Result<()> {
    use audioadapter_buffers::direct::InterleavedSlice;

    while in_buf.len() / channels >= resampler.input_frames_next() {
        let in_frames = resampler.input_frames_next();
        let in_elems = in_frames * channels;

        let input = InterleavedSlice::new(&in_buf[..in_elems], channels, in_frames)
            .map_err(|_| crate::AudioError::Decode("buffer error".into()))?;

        let out_cap = resampler.process_all_needed_output_len(in_frames);
        let mut out_buf = vec![0.0; out_cap * channels];
        let mut output = InterleavedSlice::new_mut(&mut out_buf, channels, out_cap)
            .map_err(|_| crate::AudioError::Decode("buffer error".into()))?;

        match resampler.process_into_buffer(&input, &mut output, None) {
            Ok((nbr_in, nbr_out)) => {
                push_samples(prod, &out_buf[..nbr_out * channels]);
                in_buf.drain(..nbr_in * channels);
            }
            Err(e) => return Err(crate::AudioError::Decode(e.to_string())),
        }
    }
    Ok(())
}

fn push_samples(prod: &mut <RingBuf as ringbuf::traits::Split>::Prod, samples: &[Sample]) {
    let mut start = 0;
    while start < samples.len() {
        let pushed = prod.push_slice(&samples[start..]);
        if pushed == 0 {
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        start += pushed;
    }
}
