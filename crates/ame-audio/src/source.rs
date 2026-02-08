use std::io::{self, Read, Seek, SeekFrom};
use std::time::Duration;

use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::default::get_probe;

pub trait Source: Send {
    fn total_duration(&self) -> Option<Duration>;
    fn is_network(&self) -> bool;
    fn is_seekable(&self) -> bool;
    fn seek(&mut self, position: Duration) -> io::Result<()>;
    fn current_position(&self) -> Duration;
    fn into_media_source(self: Box<Self>) -> Box<dyn symphonia::core::io::MediaSource>;
}

pub struct FileSource {
    file: std::fs::File,
    path: String,
    duration: Option<Duration>,
    current_pos: Duration,
}

impl FileSource {
    pub fn new(path: &str) -> crate::Result<Self> {
        let file = std::fs::File::open(path)?;
        let duration = probe_duration(path);
        Ok(Self {
            file,
            path: path.to_string(),
            duration,
            current_pos: Duration::ZERO,
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn reopen(&self) -> crate::Result<Self> {
        Self::new(&self.path)
    }
}

impl Source for FileSource {
    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }

    fn is_network(&self) -> bool {
        false
    }

    fn is_seekable(&self) -> bool {
        true
    }

    fn seek(&mut self, position: Duration) -> io::Result<()> {
        // Note: This is byte-level seek. For accurate audio seek,
        // the decoder will use FormatReader::seek() for time-based seek.
        self.file.seek(SeekFrom::Start(0))?;
        self.current_pos = position;
        Ok(())
    }

    fn current_position(&self) -> Duration {
        self.current_pos
    }

    fn into_media_source(self: Box<Self>) -> Box<dyn symphonia::core::io::MediaSource> {
        Box::new(self.file)
    }
}

pub struct NetworkSource {
    reader: Box<dyn Read + Send + Sync>,
    url: String,
    supports_range: bool,
    current_pos: Duration,
}

impl NetworkSource {
    pub fn new(reader: Box<dyn Read + Send + Sync>) -> Self {
        Self {
            reader,
            url: String::new(),
            supports_range: false,
            current_pos: Duration::ZERO,
        }
    }

    pub fn with_url(
        reader: Box<dyn Read + Send + Sync>,
        url: String,
        supports_range: bool,
    ) -> Self {
        Self {
            reader,
            url,
            supports_range,
            current_pos: Duration::ZERO,
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

fn probe_duration(path: &str) -> Option<Duration> {
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let probed = get_probe()
        .format(
            &Default::default(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let format = probed.format;
    let track = format.default_track()?;
    let codec_params = &track.codec_params;

    // Calculate duration from n_frames and sample_rate
    let n_frames = codec_params.n_frames?;
    let sample_rate = codec_params.sample_rate?;

    let secs = n_frames / sample_rate as u64;
    let nanos = ((n_frames % sample_rate as u64) * 1_000_000_000 / sample_rate as u64) as u32;

    Some(Duration::new(secs, nanos))
}

impl Read for NetworkSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

impl Seek for NetworkSource {
    fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
        // NetworkSource uses HTTP Range for seek, handled by recreating the stream
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "use seek() method on Source trait for network streams",
        ))
    }
}

impl symphonia::core::io::MediaSource for NetworkSource {
    fn is_seekable(&self) -> bool {
        self.supports_range
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

impl Source for NetworkSource {
    fn total_duration(&self) -> Option<Duration> {
        None
    }

    fn is_network(&self) -> bool {
        true
    }

    fn is_seekable(&self) -> bool {
        self.supports_range
    }

    fn seek(&mut self, position: Duration) -> io::Result<()> {
        if !self.supports_range {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "server does not support range requests",
            ));
        }
        self.current_pos = position;
        Ok(())
    }

    fn current_position(&self) -> Duration {
        self.current_pos
    }

    fn into_media_source(self: Box<Self>) -> Box<dyn symphonia::core::io::MediaSource> {
        self
    }
}
