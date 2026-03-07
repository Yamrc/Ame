use thiserror::Error;

pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AudioError {
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidStateTransition {
        from: crate::EngineState,
        to: crate::EngineState,
    },
    #[error("source open failed: {reason}")]
    SourceOpenFailed { reason: String },
    #[error("unsupported seek for current source")]
    UnsupportedSeek,
    #[error("network error: {reason}")]
    Network { reason: String },
    #[error("http status error: {code} ({url})")]
    HttpStatus { code: u16, url: String },
    #[error("decode failed: {reason}")]
    DecodeFailed { reason: String },
    #[error("output init failed: {reason}")]
    OutputInitFailed { reason: String },
    #[error("output device lost: {reason}")]
    DeviceLost { reason: String },
    #[error("backend unavailable: {backend:?}")]
    BackendUnavailable { backend: crate::OutputBackendKind },
    #[error("device not available: {device}")]
    DeviceNotAvailable { device: String },
    #[error("config invalid: {reason}")]
    ConfigInvalid { reason: String },
    #[error("runtime channel closed")]
    ChannelClosed,
    #[error("runtime join failed: {reason}")]
    RuntimeJoinFailed { reason: String },
}

impl From<std::io::Error> for AudioError {
    fn from(value: std::io::Error) -> Self {
        Self::SourceOpenFailed {
            reason: value.to_string(),
        }
    }
}

impl From<cpal::BuildStreamError> for AudioError {
    fn from(value: cpal::BuildStreamError) -> Self {
        Self::OutputInitFailed {
            reason: value.to_string(),
        }
    }
}

impl From<cpal::PlayStreamError> for AudioError {
    fn from(value: cpal::PlayStreamError) -> Self {
        Self::OutputInitFailed {
            reason: value.to_string(),
        }
    }
}

impl From<cpal::PauseStreamError> for AudioError {
    fn from(value: cpal::PauseStreamError) -> Self {
        Self::OutputInitFailed {
            reason: value.to_string(),
        }
    }
}

impl From<cpal::DefaultStreamConfigError> for AudioError {
    fn from(value: cpal::DefaultStreamConfigError) -> Self {
        Self::OutputInitFailed {
            reason: value.to_string(),
        }
    }
}
