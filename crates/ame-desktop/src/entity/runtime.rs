use ame_core::credential::CredentialStore;
use ame_core::storage::{AppStorage, CacheIndexStore, SettingsStore, StateStore};
use nekowg::{AppContext, Context, Entity, Global};
use serde::{Deserialize, Serialize};

use crate::entity::app::{AppEntity, CloseBehavior, HomeArtistLanguage, ShellState};
use crate::entity::pages::{
    DiscoverPageState, HomePageState, LibraryPageState, LoginPageState, PlaylistPageState,
    SearchPageState,
};
use crate::entity::player::{PlaybackMode, PlayerEntity, QueueItem};
use crate::entity::session::SessionState;

pub const KEY_PLAYER_VOLUME: &str = "player.volume";
pub const KEY_PLAYER_MODE: &str = "player.mode";
pub const KEY_PLAYER_QUEUE: &str = "player.queue";
pub const KEY_PLAYER_CURRENT_INDEX: &str = "player.current_index";
pub const KEY_PLAYER_POSITION_MS: &str = "player.position_ms";
pub const KEY_PLAYER_DURATION_MS: &str = "player.duration_ms";
pub const KEY_PLAYER_WAS_PLAYING: &str = "player.was_playing";
pub const KEY_WINDOW_CLOSE_BEHAVIOR: &str = "window.close_behavior";
pub const KEY_HOME_ARTIST_LANGUAGE: &str = "home.artist_language";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedQueueItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artist: String,
    pub cover_url: Option<String>,
}

#[derive(Clone, Default)]
pub struct AppServices {
    pub settings_store: Option<SettingsStore>,
    pub state_store: Option<StateStore>,
    pub cache_store: Option<CacheIndexStore>,
    pub credential_store: CredentialStore,
}

#[derive(Clone)]
pub struct AppRuntime {
    pub services: AppServices,
    pub app: Entity<AppEntity>,
    pub player: Entity<PlayerEntity>,
    pub shell: Entity<ShellState>,
    pub session: Entity<SessionState>,
    pub home: Entity<HomePageState>,
    pub discover: Entity<DiscoverPageState>,
    pub library: Entity<LibraryPageState>,
    pub search: Entity<SearchPageState>,
    pub playlist: Entity<PlaylistPageState>,
    pub login: Entity<LoginPageState>,
}

impl Global for AppRuntime {}

pub struct RuntimeBootstrap {
    pub runtime: AppRuntime,
}

impl AppRuntime {
    pub fn bootstrap<T>(cx: &mut Context<T>) -> RuntimeBootstrap {
        let mut startup_error = None;
        let mut player_state = PlayerEntity::default();
        player_state.mode = PlaybackMode::Sequence;

        let mut services = AppServices::default();
        let mut persisted_was_playing = false;
        let mut close_behavior = CloseBehavior::default();
        let mut home_artist_language = HomeArtistLanguage::default();

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
                            Err(err) => push_message(
                                &mut startup_error,
                                format!("打开 state 存储失败: {err}"),
                            ),
                        }
                        match storage.cache() {
                            Ok(cache) => services.cache_store = Some(cache),
                            Err(err) => push_message(
                                &mut startup_error,
                                format!("打开 cache 存储失败: {err}"),
                            ),
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
                Err(err) => {
                    push_message(&mut startup_error, format!("读取首页艺人语种失败: {err}"))
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
            match state.get::<bool>(KEY_PLAYER_WAS_PLAYING) {
                Ok(Some(value)) => persisted_was_playing = value,
                Ok(None) => {}
                Err(err) => push_message(&mut startup_error, format!("读取播放状态失败: {err}")),
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

        let mut login_state = LoginPageState::default();
        if persisted_was_playing {
            login_state.qr_status = Some("已恢复上次播放状态（未自动播放）".to_string());
        }

        let runtime = Self {
            services,
            app: cx.new(move |_| AppEntity {
                search_query: String::new(),
                home_artist_language,
            }),
            player: cx.new(move |_| player_state.clone()),
            shell: cx.new(move |_| shell_state.clone()),
            session: cx.new(move |_| session_state.clone()),
            home: cx.new(|_| HomePageState::default()),
            discover: cx.new(|_| DiscoverPageState::default()),
            library: cx.new(|_| LibraryPageState::default()),
            search: cx.new(|_| SearchPageState::default()),
            playlist: cx.new(|_| PlaylistPageState::default()),
            login: cx.new(move |_| login_state.clone()),
        };

        cx.set_global(runtime.clone());

        RuntimeBootstrap { runtime }
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
