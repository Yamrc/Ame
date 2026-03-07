use crate::{AudioError, EngineState, OutputBackendKind};

#[derive(Debug, Clone)]
pub enum AudioEvent {
    StateChanged {
        from: EngineState,
        to: EngineState,
    },
    TrackEnded,
    DeviceChanged {
        backend: OutputBackendKind,
        device: Option<String>,
    },
    DeviceLost {
        backend: OutputBackendKind,
        device: Option<String>,
    },
    Error(AudioError),
}
