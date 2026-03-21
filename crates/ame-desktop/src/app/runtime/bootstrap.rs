use std::sync::{Arc, Mutex};

use ame_audio::{AudioCommand, AudioConfig, AudioService};
use ame_core::storage::AppStorage;
use nekowg::{AppContext, Context};

use crate::app::audio_bridge::AudioBridgeEntity;
use crate::app::env::AppEnv;
use crate::app::state::AppEntity;
use crate::domain::cache::CacheService;
use crate::domain::player::{PlaybackMode, PlayerEntity, QueueItem};
use crate::domain::session::{PersistedSessionIdentity, SessionState};
use crate::domain::settings::{CloseBehavior, HomeArtistLanguage};
use crate::domain::shell::ShellState;

use super::keys::{
    KEY_HOME_ARTIST_LANGUAGE, KEY_PLAYER_CURRENT_INDEX, KEY_PLAYER_DURATION_MS, KEY_PLAYER_MODE,
    KEY_PLAYER_POSITION_MS, KEY_PLAYER_QUEUE, KEY_PLAYER_VOLUME, KEY_PLAYER_WAS_PLAYING,
    KEY_SESSION_IDENTITY, KEY_WINDOW_CLOSE_BEHAVIOR,
};
use super::{AppRuntime, AppServices, PersistedQueueItem, RuntimeBootstrap};

pub(super) fn bootstrap_runtime<T>(cx: &mut Context<T>) -> RuntimeBootstrap {
    let mut startup_error = None;
    let mut player_state = PlayerEntity::default();
    player_state.mode = PlaybackMode::Sequence;

    let mut services = AppServices::default();
    let mut close_behavior = CloseBehavior::default();
    let mut home_artist_language = HomeArtistLanguage::default();
    let (audio_bridge, audio_runtime, audio_error) =
        match AudioService::spawn(AudioConfig::default()) {
            Ok((service, runtime)) => (Some(AudioBridgeEntity::new(service)), Some(runtime), None),
            Err(err) => (
                None,
                None,
                Some(format!("Failed to initialize audio: {err}")),
            ),
        };
    if let Some(bridge) = audio_bridge {
        services.audio_bridge = Some(Arc::new(Mutex::new(bridge)));
    }

    if let Some(base_dir) = dirs::data_local_dir() {
        let db_path = base_dir.join("ame");
        match std::fs::create_dir_all(&db_path) {
            Ok(_) => match AppStorage::open(&db_path) {
                Ok(storage) => {
                    services.settings_store = Some(storage.settings());
                    services.state_store = Some(storage.state());
                    services.network_cache = Some(Arc::new(CacheService::new(
                        storage.firework(),
                        storage.weather(),
                        storage.geological(),
                        storage.response_dir().to_path_buf(),
                    )));

                    if let Some(network_cache) = services.network_cache.as_ref()
                        && let Err(err) = network_cache.run_maintenance()
                    {
                        push_message(
                            &mut startup_error,
                            format!("Failed to run network cache maintenance: {err}"),
                        );
                    }
                }
                Err(err) => {
                    push_message(&mut startup_error, format!("Failed to open storage: {err}"))
                }
            },
            Err(err) => push_message(
                &mut startup_error,
                format!("Failed to create data directory: {err}"),
            ),
        }
    } else {
        push_message(
            &mut startup_error,
            "Failed to locate the system data directory".to_string(),
        );
    }

    if let Some(settings) = services.settings_store.as_ref() {
        match settings.get::<f32>(KEY_PLAYER_VOLUME) {
            Ok(Some(volume)) => player_state.set_volume(volume),
            Ok(None) => {}
            Err(err) => push_message(&mut startup_error, format!("Failed to read volume: {err}")),
        }
        match settings.get::<PlaybackMode>(KEY_PLAYER_MODE) {
            Ok(Some(mode)) => player_state.mode = mode,
            Ok(None) => {}
            Err(err) => push_message(
                &mut startup_error,
                format!("Failed to read playback mode: {err}"),
            ),
        }
        match settings.get::<CloseBehavior>(KEY_WINDOW_CLOSE_BEHAVIOR) {
            Ok(Some(value)) => close_behavior = value,
            Ok(None) => {}
            Err(err) => push_message(
                &mut startup_error,
                format!("Failed to read close behavior: {err}"),
            ),
        }
        match settings.get::<HomeArtistLanguage>(KEY_HOME_ARTIST_LANGUAGE) {
            Ok(Some(value)) => home_artist_language = value,
            Ok(None) => {}
            Err(err) => push_message(
                &mut startup_error,
                format!("Failed to read home artist language: {err}"),
            ),
        }
    }

    if let Some(audio_bridge) = services.audio_bridge.as_ref() {
        match audio_bridge.lock() {
            Ok(bridge) => {
                if let Err(err) = bridge.send(AudioCommand::SetVolume(player_state.volume)) {
                    push_message(&mut startup_error, format!("Failed to set volume: {err}"));
                }
            }
            Err(err) => {
                push_message(
                    &mut startup_error,
                    format!("Failed to lock audio bridge: {err}"),
                );
            }
        }
    }

    if let Some(state) = services.state_store.as_ref() {
        match state.get::<Vec<PersistedQueueItem>>(KEY_PLAYER_QUEUE) {
            Ok(Some(queue)) => {
                player_state.set_queue(
                    queue
                        .into_iter()
                        .map(|item| QueueItem {
                            id: item.id,
                            name: item.name,
                            alias: item.alias,
                            artist: item.artist,
                            album: item.album,
                            duration_ms: item.duration_ms,
                            cover_url: item.cover_url,
                            source_url: None,
                        })
                        .collect(),
                );
            }
            Ok(None) => {}
            Err(err) => push_message(&mut startup_error, format!("Failed to read queue: {err}")),
        }
        match state.get::<Option<usize>>(KEY_PLAYER_CURRENT_INDEX) {
            Ok(Some(index)) => {
                player_state.current_index = index.filter(|i| *i < player_state.queue.len());
            }
            Ok(None) => {}
            Err(err) => push_message(
                &mut startup_error,
                format!("Failed to read current index: {err}"),
            ),
        }
        match state.get::<u64>(KEY_PLAYER_POSITION_MS) {
            Ok(Some(position)) => player_state.position_ms = position,
            Ok(None) => {}
            Err(err) => push_message(
                &mut startup_error,
                format!("Failed to read playback position: {err}"),
            ),
        }
        match state.get::<u64>(KEY_PLAYER_DURATION_MS) {
            Ok(Some(duration)) => player_state.duration_ms = duration,
            Ok(None) => {}
            Err(err) => push_message(
                &mut startup_error,
                format!("Failed to read duration: {err}"),
            ),
        }
        if let Err(err) = state.get::<bool>(KEY_PLAYER_WAS_PLAYING) {
            push_message(
                &mut startup_error,
                format!("Failed to read playback state: {err}"),
            );
        }
    }

    player_state.is_playing = false;

    let mut session_state = SessionState::default();
    match services.credential_store.load_auth_bundle() {
        Ok(Some(bundle)) => session_state.auth_bundle = bundle,
        Ok(None) => {}
        Err(err) => push_message(
            &mut startup_error,
            format!("Failed to read keyring credentials: {err}"),
        ),
    }
    if let Some(state_store) = services.state_store.as_ref() {
        match state_store.get::<PersistedSessionIdentity>(KEY_SESSION_IDENTITY) {
            Ok(Some(identity)) if identity.matches_bundle(&session_state.auth_bundle) => {
                identity.apply_to_session(&mut session_state);
            }
            Ok(Some(_)) | Ok(None) => {}
            Err(err) => push_message(
                &mut startup_error,
                format!("Failed to read session identity snapshot: {err}"),
            ),
        }
    }

    let mut shell_state = ShellState {
        error: None,
        close_behavior,
    };
    if let Some(err) = startup_error {
        push_message(&mut shell_state.error, err);
    }
    if let Some(err) = audio_error {
        push_message(&mut shell_state.error, err);
    }

    let runtime = AppRuntime {
        services,
        app: cx.new(move |_| AppEntity {
            search_query: String::new(),
            home_artist_language,
        }),
        player: cx.new(move |_| player_state.clone()),
        shell: cx.new(move |_| shell_state.clone()),
        session: cx.new(move |_| session_state.clone()),
    };

    let env = AppEnv {
        app: runtime.app.clone(),
        player: runtime.player.clone(),
        shell: runtime.shell.clone(),
        session: runtime.session.clone(),
    };

    cx.set_global(env.clone());
    cx.set_global(runtime.clone());

    RuntimeBootstrap {
        env,
        runtime,
        audio_runtime,
    }
}

fn push_message(slot: &mut Option<String>, message: String) {
    if message.trim().is_empty() {
        return;
    }
    match slot {
        Some(existing) => {
            existing.push('\n');
            existing.push_str(&message);
        }
        None => *slot = Some(message),
    }
}
