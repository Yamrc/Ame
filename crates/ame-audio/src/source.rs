use std::io::{Read, Seek, SeekFrom};
use std::time::Duration;

pub trait Source: Send {
    fn total_duration(&self) -> Option<Duration>;
    fn is_network(&self) -> bool;
    fn into_media_source(self: Box<Self>) -> Box<dyn symphonia::core::io::MediaSource>;
}

pub struct FileSource {
    file: std::fs::File,
    duration: Option<Duration>,
}

impl FileSource {
    pub fn new(path: &str) -> crate::Result<Self> {
        let file = std::fs::File::open(path)?;
        Ok(Self {
            file,
            duration: None,
        })
    }
}

impl Source for FileSource {
    fn total_duration(&self) -> Option<Duration> {
        self.duration
    }
    fn is_network(&self) -> bool {
        false
    }
    fn into_media_source(self: Box<Self>) -> Box<dyn symphonia::core::io::MediaSource> {
        Box::new(self.file)
    }
}

pub struct NetworkSource {
    reader: Box<dyn Read + Send + Sync>,
    buffer: Vec<u8>,
    position: u64,
}

impl NetworkSource {
    pub fn new(reader: Box<dyn Read + Send + Sync>) -> Self {
        Self {
            reader,
            buffer: Vec::with_capacity(64 * 1024),
            position: 0,
        }
    }
}

impl Read for NetworkSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}

impl Seek for NetworkSource {
    fn seek(&mut self, _pos: SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "network stream not seekable",
        ))
    }
}

impl symphonia::core::io::MediaSource for NetworkSource {
    fn is_seekable(&self) -> bool {
        false
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

    fn into_media_source(self: Box<Self>) -> Box<dyn symphonia::core::io::MediaSource> {
        self
    }
}
