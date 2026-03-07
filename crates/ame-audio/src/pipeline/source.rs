use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, RANGE};
use symphonia::core::io::MediaSource;

use crate::config::NetworkConfig;
use crate::error::{AudioError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceSpec {
    LocalFile(PathBuf),
    NetworkUrl(String),
}

impl SourceSpec {
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self::LocalFile(path.into())
    }

    pub fn network(url: impl Into<String>) -> Self {
        Self::NetworkUrl(url.into())
    }

    pub fn describe(&self) -> String {
        match self {
            SourceSpec::LocalFile(path) => path.display().to_string(),
            SourceSpec::NetworkUrl(url) => url.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekTarget {
    Milliseconds(u64),
}

impl SeekTarget {
    pub fn ms(ms: u64) -> Self {
        Self::Milliseconds(ms)
    }

    pub fn to_millis(self) -> u64 {
        match self {
            SeekTarget::Milliseconds(ms) => ms,
        }
    }
}

pub struct OpenedSource {
    pub media_source: Box<dyn MediaSource>,
    pub seekable: bool,
}

pub trait SourceFactory: Send + Sync + 'static {
    fn open(&self, spec: &SourceSpec) -> Result<OpenedSource>;
}

#[derive(Clone)]
pub struct DefaultSourceFactory {
    client: Client,
    network: NetworkConfig,
}

impl DefaultSourceFactory {
    pub fn new(network: NetworkConfig) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(network.connect_timeout_ms))
            .build()
            .map_err(|err| AudioError::Network {
                reason: err.to_string(),
            })?;
        Ok(Self { client, network })
    }
}

impl SourceFactory for DefaultSourceFactory {
    fn open(&self, spec: &SourceSpec) -> Result<OpenedSource> {
        match spec {
            SourceSpec::LocalFile(path) => {
                let file =
                    std::fs::File::open(path).map_err(|err| AudioError::SourceOpenFailed {
                        reason: format!("{} ({})", path.display(), err),
                    })?;
                Ok(OpenedSource {
                    media_source: Box::new(file),
                    seekable: true,
                })
            }
            SourceSpec::NetworkUrl(url) => {
                let source = RangeHttpSource::new(self.client.clone(), url, self.network.clone())?;
                let seekable = source.is_seekable();
                Ok(OpenedSource {
                    media_source: Box::new(source),
                    seekable,
                })
            }
        }
    }
}

struct RangeHttpSource {
    client: Client,
    url: String,
    network: NetworkConfig,
    reader: Mutex<Option<Response>>,
    position: u64,
    content_length: Option<u64>,
    supports_range: bool,
}

impl RangeHttpSource {
    fn map_http_status(url: &str, status: StatusCode, context: &str) -> AudioError {
        let _ = context;
        AudioError::HttpStatus {
            code: status.as_u16(),
            url: url.to_string(),
        }
    }

    fn validate_stream_response(url: &str, offset: u64, resp: Response) -> Result<Response> {
        let status = resp.status();
        if offset > 0 && status == StatusCode::OK {
            return Err(AudioError::UnsupportedSeek);
        }

        if status.is_success() {
            return Ok(resp);
        }

        Err(Self::map_http_status(url, status, "GET"))
    }

    fn new(client: Client, url: &str, network: NetworkConfig) -> Result<Self> {
        let head = client
            .head(url)
            .timeout(Duration::from_millis(network.timeout_ms))
            .send()
            .map_err(|err| AudioError::Network {
                reason: format!("HEAD {} failed: {err}", url),
            })?;
        if !head.status().is_success() {
            return Err(Self::map_http_status(url, head.status(), "HEAD"));
        }

        let supports_range = head
            .headers()
            .get(ACCEPT_RANGES)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.eq_ignore_ascii_case("bytes"));

        let content_length = head
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());

        let mut source = Self {
            client,
            url: url.to_string(),
            network,
            reader: Mutex::new(None),
            position: 0,
            content_length,
            supports_range,
        };
        source.reopen_at(0)?;
        Ok(source)
    }

    fn reopen_at(&mut self, offset: u64) -> Result<()> {
        let mut last_error = None;
        for _ in 0..=self.network.max_retries {
            let mut request = self.client.get(&self.url);
            request = request.header(RANGE, format!("bytes={offset}-"));

            match request.send() {
                Ok(resp) => {
                    let status = resp.status();
                    let has_content_range =
                        resp.headers().contains_key(reqwest::header::CONTENT_RANGE);
                    let resp = Self::validate_stream_response(&self.url, offset, resp)?;
                    if status == StatusCode::PARTIAL_CONTENT || has_content_range {
                        self.supports_range = true;
                    }
                    self.position = offset;
                    let mut guard = self.reader.lock().map_err(|_| AudioError::Network {
                        reason: "network reader lock poisoned".into(),
                    })?;
                    *guard = Some(resp);
                    return Ok(());
                }
                Err(err) => {
                    last_error = Some(err.to_string());
                }
            }
        }

        Err(AudioError::Network {
            reason: last_error.unwrap_or_else(|| "request failed".into()),
        })
    }

    fn compute_seek_target(&self, pos: SeekFrom) -> Result<u64> {
        let target = match pos {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(delta) => self.position as i128 + delta as i128,
            SeekFrom::End(delta) => {
                let len = self.content_length.ok_or(AudioError::UnsupportedSeek)? as i128;
                len + delta as i128
            }
        };

        if target < 0 {
            return Err(AudioError::UnsupportedSeek);
        }

        Ok(target as u64)
    }
}

impl Read for RangeHttpSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        for attempt in 0..=self.network.max_retries {
            let read_result = {
                let mut guard = self
                    .reader
                    .lock()
                    .map_err(|_| std::io::Error::other("network reader lock poisoned"))?;
                let Some(reader) = guard.as_mut() else {
                    return Ok(0);
                };
                reader.read(buf)
            };

            match read_result {
                Ok(read) => {
                    if read > 0 {
                        self.position = self.position.saturating_add(read as u64);
                        return Ok(read);
                    }
                    let maybe_truncated = self
                        .content_length
                        .is_some_and(|length| self.position < length);
                    if !maybe_truncated || attempt >= self.network.max_retries {
                        return Ok(0);
                    }
                }
                Err(err) => {
                    if attempt >= self.network.max_retries {
                        return Err(err);
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(100));
            self.reopen_at(self.position).map_err(|err| match err {
                AudioError::HttpStatus { code: 403, .. } => {
                    std::io::Error::new(std::io::ErrorKind::PermissionDenied, err.to_string())
                }
                _ => std::io::Error::other(err.to_string()),
            })?;
        }

        Ok(0)
    }
}

impl Seek for RangeHttpSource {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target = self
            .compute_seek_target(pos)
            .map_err(|err| std::io::Error::other(err.to_string()))?;

        if target == self.position {
            return Ok(self.position);
        }

        self.reopen_at(target).map_err(|err| match err {
            AudioError::UnsupportedSeek => {
                std::io::Error::new(std::io::ErrorKind::Unsupported, err.to_string())
            }
            _ => std::io::Error::other(err.to_string()),
        })?;
        Ok(self.position)
    }
}

impl MediaSource for RangeHttpSource {
    fn is_seekable(&self) -> bool {
        self.supports_range
    }

    fn byte_len(&self) -> Option<u64> {
        self.content_length
    }
}

#[derive(Debug, Clone)]
pub struct FileSource {
    path: PathBuf,
}

impl FileSource {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(AudioError::SourceOpenFailed {
                reason: format!("{} does not exist", path.display()),
            });
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct NetworkSource {
    url: String,
}

impl NetworkSource {
    pub fn from_http(url: impl Into<String>) -> Result<Self> {
        let url = url.into();
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(AudioError::SourceOpenFailed {
                reason: "network url must start with http:// or https://".into(),
            });
        }
        Ok(Self { url })
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

pub trait Source: Send {
    fn into_spec(self: Box<Self>) -> SourceSpec;
}

impl Source for FileSource {
    fn into_spec(self: Box<Self>) -> SourceSpec {
        SourceSpec::LocalFile(self.path)
    }
}

impl Source for NetworkSource {
    fn into_spec(self: Box<Self>) -> SourceSpec {
        SourceSpec::NetworkUrl(self.url)
    }
}
