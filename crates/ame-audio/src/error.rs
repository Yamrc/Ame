use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("CPAL build error: {0}")]
    CpalBuild(#[from] cpal::BuildStreamError),
    #[error("CPAL play error: {0}")]
    CpalPlay(#[from] cpal::PlayStreamError),
    #[error("CPAL pause error: {0}")]
    CpalPause(#[from] cpal::PauseStreamError),
    #[error("CPAL config error: {0}")]
    CpalConfig(#[from] cpal::DefaultStreamConfigError),
    #[error("Device not available")]
    DeviceNotAvailable,
    #[error("Decode error: {0}")]
    Decode(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unsupported format")]
    UnsupportedFormat,
}

pub type Result<T> = std::result::Result<T, AudioError>;
