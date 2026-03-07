use crate::{EngineState, OutputBackendKind, SourceSpec};

#[derive(Debug, Clone)]
pub struct AudioSnapshot {
    pub state: EngineState,
    pub is_playing: bool,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub backend: OutputBackendKind,
    pub device: Option<String>,
    pub source: Option<SourceSpec>,
}

impl Default for AudioSnapshot {
    fn default() -> Self {
        Self {
            state: EngineState::Idle,
            is_playing: false,
            position_ms: 0,
            duration_ms: 0,
            volume: 0.7,
            backend: OutputBackendKind::PlatformDefault,
            device: None,
            source: None,
        }
    }
}
