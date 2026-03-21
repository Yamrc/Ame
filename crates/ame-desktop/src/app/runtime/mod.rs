mod bootstrap;
mod keys;

use std::sync::{Arc, Mutex};

use ame_audio::AudioRuntimeHandle;
use ame_core::credential::CredentialStore;
use ame_core::storage::{SettingsStorage, StateStorage};
use nekowg::{Context, Entity, Global};
use serde::{Deserialize, Serialize};

use crate::app::audio_bridge::AudioBridgeEntity;
use crate::app::env::AppEnv;
use crate::app::state::AppEntity;
use crate::domain::cache::CacheService;
use crate::domain::player::PlayerEntity;
use crate::domain::session::SessionState;
use crate::domain::shell::ShellState;

pub use keys::{
    KEY_HOME_ARTIST_LANGUAGE, KEY_PLAYER_CURRENT_INDEX, KEY_PLAYER_DURATION_MS, KEY_PLAYER_MODE,
    KEY_PLAYER_POSITION_MS, KEY_PLAYER_QUEUE, KEY_PLAYER_VOLUME, KEY_PLAYER_WAS_PLAYING,
    KEY_SESSION_IDENTITY, KEY_WINDOW_CLOSE_BEHAVIOR,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedQueueItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artist: String,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

#[derive(Clone, Default)]
pub struct AppServices {
    pub settings_store: Option<SettingsStorage>,
    pub state_store: Option<StateStorage>,
    pub network_cache: Option<Arc<CacheService>>,
    pub credential_store: CredentialStore,
    pub audio_bridge: Option<Arc<Mutex<AudioBridgeEntity>>>,
}

#[derive(Clone)]
pub struct AppRuntime {
    pub services: AppServices,
    pub app: Entity<AppEntity>,
    pub player: Entity<PlayerEntity>,
    pub shell: Entity<ShellState>,
    pub session: Entity<SessionState>,
}

impl Global for AppRuntime {}

pub struct RuntimeBootstrap {
    pub env: AppEnv,
    pub runtime: AppRuntime,
    pub audio_runtime: Option<AudioRuntimeHandle>,
}

impl AppRuntime {
    pub fn bootstrap<T>(cx: &mut Context<T>) -> RuntimeBootstrap {
        bootstrap::bootstrap_runtime(cx)
    }
}
