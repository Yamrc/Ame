#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleQualityPreset {
    LowLatency,
    Balanced,
    HighQuality,
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputBackendKind {
    PlatformDefault,
    Wasapi,
    Asio,
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub timeout_ms: u64,
    pub connect_timeout_ms: u64,
    pub max_retries: usize,
    pub prebuffer_bytes: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            connect_timeout_ms: 5_000,
            max_retries: 4,
            prebuffer_bytes: 64 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub backend: OutputBackendKind,
    pub preferred_device: Option<String>,
    pub snapshot_hz: u32,
    pub volume: f32,
    pub resample_quality: ResampleQualityPreset,
    pub network: NetworkConfig,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            backend: OutputBackendKind::PlatformDefault,
            preferred_device: None,
            snapshot_hz: 30,
            volume: 0.7,
            resample_quality: ResampleQualityPreset::Balanced,
            network: NetworkConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeConfigPatch {
    pub backend: Option<OutputBackendKind>,
    pub preferred_device: Option<Option<String>>,
    pub snapshot_hz: Option<u32>,
    pub volume: Option<f32>,
    pub resample_quality: Option<ResampleQualityPreset>,
    pub network: Option<NetworkConfig>,
}

impl AudioConfig {
    pub fn apply_patch(&mut self, patch: RuntimeConfigPatch) {
        if let Some(backend) = patch.backend {
            self.backend = backend;
        }
        if let Some(device) = patch.preferred_device {
            self.preferred_device = device;
        }
        if let Some(snapshot_hz) = patch.snapshot_hz {
            self.snapshot_hz = snapshot_hz.max(1);
        }
        if let Some(volume) = patch.volume {
            self.volume = volume.clamp(0.0, 1.0);
        }
        if let Some(quality) = patch.resample_quality {
            self.resample_quality = quality;
        }
        if let Some(network) = patch.network {
            self.network = network;
        }
    }
}
