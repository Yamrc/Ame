use std::sync::{Arc, Mutex};

use ame_audio::{AudioCommand, AudioConfig, AudioService};
use ame_core::storage::AppStorage;
use nekowg::{AppContext, Context};

use crate::app::audio_bridge::AudioBridgeEntity;
use crate::app::env::AppEnv;
use crate::app::state::AppEntity;
use crate::domain::player::{PlaybackMode, PlayerEntity, QueueItem};
use crate::domain::session::SessionState;
use crate::domain::settings::{CloseBehavior, HomeArtistLanguage};
use crate::domain::shell::ShellState;

use super::keys::{
    KEY_HOME_ARTIST_LANGUAGE, KEY_PLAYER_CURRENT_INDEX, KEY_PLAYER_DURATION_MS, KEY_PLAYER_MODE,
    KEY_PLAYER_POSITION_MS, KEY_PLAYER_QUEUE, KEY_PLAYER_VOLUME, KEY_PLAYER_WAS_PLAYING,
    KEY_WINDOW_CLOSE_BEHAVIOR,
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
            Err(err) => (None, None, Some(format!("音频初始化失败: {err}"))),
        };
    if let Some(bridge) = audio_bridge {
        services.audio_bridge = Some(Arc::new(Mutex::new(bridge)));
    }

    if let Some(base_dir) = dirs::data_local_dir() {
        let db_path = base_dir.join("ame");
        match std::fs::create_dir_all(&db_path) {
            Ok(_) => match AppStorage::open(&db_path) {
                Ok(storage) => {
                    match storage.settings() {
                        Ok(settings) => services.settings_store = Some(settings),
                        Err(err) => push_message(
                            &mut startup_error,
                            format!("打开 settings 存储失败: {err}"),
                        ),
                    }
                    match storage.state() {
                        Ok(state) => services.state_store = Some(state),
                        Err(err) => {
                            push_message(&mut startup_error, format!("打开 state 存储失败: {err}"))
                        }
                    }
                    match storage.cache() {
                        Ok(cache) => services.cache_store = Some(cache),
                        Err(err) => {
                            push_message(&mut startup_error, format!("打开 cache 存储失败: {err}"))
                        }
                    }
                }
                Err(err) => push_message(&mut startup_error, format!("打开存储失败: {err}")),
            },
            Err(err) => push_message(&mut startup_error, format!("创建数据目录失败: {err}")),
        }
    } else {
        push_message(&mut startup_error, "无法定位系统数据目录".to_string());
    }

    if let Some(settings) = services.settings_store.as_ref() {
        match settings.get::<f32>(KEY_PLAYER_VOLUME) {
            Ok(Some(volume)) => player_state.set_volume(volume),
            Ok(None) => {}
            Err(err) => push_message(&mut startup_error, format!("读取音量失败: {err}")),
        }
        match settings.get::<PlaybackMode>(KEY_PLAYER_MODE) {
            Ok(Some(mode)) => player_state.mode = mode,
            Ok(None) => {}
            Err(err) => push_message(&mut startup_error, format!("读取播放模式失败: {err}")),
        }
        match settings.get::<CloseBehavior>(KEY_WINDOW_CLOSE_BEHAVIOR) {
            Ok(Some(value)) => close_behavior = value,
            Ok(None) => {}
            Err(err) => push_message(&mut startup_error, format!("读取关闭行为失败: {err}")),
        }
        match settings.get::<HomeArtistLanguage>(KEY_HOME_ARTIST_LANGUAGE) {
            Ok(Some(value)) => home_artist_language = value,
            Ok(None) => {}
            Err(err) => push_message(&mut startup_error, format!("读取首页艺人语种失败: {err}")),
        }
    }

    if let Some(audio_bridge) = services.audio_bridge.as_ref() {
        match audio_bridge.lock() {
            Ok(bridge) => {
                if let Err(err) = bridge.send(AudioCommand::SetVolume(player_state.volume)) {
                    push_message(&mut startup_error, format!("设置音量失败: {err}"));
                }
            }
            Err(err) => {
                push_message(&mut startup_error, format!("锁定音频桥失败: {err}"));
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
            Err(err) => push_message(&mut startup_error, format!("读取队列失败: {err}")),
        }
        match state.get::<Option<usize>>(KEY_PLAYER_CURRENT_INDEX) {
            Ok(Some(index)) => {
                player_state.current_index = index.filter(|i| *i < player_state.queue.len());
            }
            Ok(None) => {}
            Err(err) => push_message(&mut startup_error, format!("读取当前索引失败: {err}")),
        }
        match state.get::<u64>(KEY_PLAYER_POSITION_MS) {
            Ok(Some(position)) => player_state.position_ms = position,
            Ok(None) => {}
            Err(err) => push_message(&mut startup_error, format!("读取播放进度失败: {err}")),
        }
        match state.get::<u64>(KEY_PLAYER_DURATION_MS) {
            Ok(Some(duration)) => player_state.duration_ms = duration,
            Ok(None) => {}
            Err(err) => push_message(&mut startup_error, format!("读取时长失败: {err}")),
        }
        if let Err(err) = state.get::<bool>(KEY_PLAYER_WAS_PLAYING) {
            push_message(&mut startup_error, format!("读取播放状态失败: {err}"));
        }
    }

    player_state.is_playing = false;

    let mut session_state = SessionState::default();
    match services.credential_store.load_auth_bundle() {
        Ok(Some(bundle)) => session_state.auth_bundle = bundle,
        Ok(None) => {}
        Err(err) => push_message(&mut startup_error, format!("读取 keyring 凭据失败: {err}")),
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
