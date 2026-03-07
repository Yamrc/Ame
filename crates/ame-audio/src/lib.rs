mod error;
mod model;
mod pipeline;
pub(crate) use model::{command, config, event, snapshot, state};
pub(crate) use pipeline::{backend, decoder, runtime, service, source};

pub use backend::{AudioDevice, OutputBackend, OutputSession, backend_for_kind};
pub use command::AudioCommand;
pub use config::{
    AudioConfig, NetworkConfig, OutputBackendKind, ResampleQualityPreset, RuntimeConfigPatch,
};
pub use error::{AudioError, Result};
pub use event::AudioEvent;
pub use service::{AudioRuntimeHandle, AudioService};
pub use snapshot::AudioSnapshot;
pub use source::{
    DefaultSourceFactory, FileSource, NetworkSource, OpenedSource, SeekTarget, Source,
    SourceFactory, SourceSpec,
};
pub use state::EngineState;

pub type Sample = f32;
