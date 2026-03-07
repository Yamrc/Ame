use crate::{RuntimeConfigPatch, SeekTarget, SourceSpec};

#[derive(Debug, Clone)]
pub enum AudioCommand {
    Open {
        source: SourceSpec,
        start_ms: u64,
        autoplay: bool,
    },
    Play,
    Pause,
    Stop,
    Seek(SeekTarget),
    SetVolume(f32),
    SwitchBackend(crate::OutputBackendKind),
    SwitchDevice(Option<String>),
    UpdateConfig(RuntimeConfigPatch),
    Shutdown,
}
