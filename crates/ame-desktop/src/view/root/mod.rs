mod handlers;
mod routes;

use crate::router;
use nekowg::{
    AnyElement, Context, Entity, Image, Render, ScrollWheelEvent, Subscription, Window, div,
    prelude::*, relative, rgb,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use ame_audio::{AudioConfig, AudioRuntimeHandle, AudioService};
use ame_core::credential::{AuthBundle, CredentialStore};
use ame_core::storage::{AppStorage, CacheIndexStore, SettingsStore, StateStore};

use crate::action::library_actions;
use crate::component::{
    bottom_bar, input,
    nav_bar::{self, NavBarActions, NavBarModel},
    scroll::{
        ScrollBarActions, ScrollBarModel, ScrollBarStyle, SmoothScrollConfig, SmoothScrollState,
    },
    slider::{self, SliderEvent, SliderStyle, SliderThumbVisibility, SliderVariant},
    theme,
    title_bar::{self, TitleBarActions, TitleBarModel},
};
use crate::entity::app::{AppEntity, CloseBehavior};
use crate::entity::audio_bridge::AudioBridgeEntity;
use crate::entity::player::{PlaybackMode, PlayerEntity, QueueItem};
use crate::kernel::{AppCommand, KernelRuntime};
use crate::view::{library, login, playlist, search};
use crate::util::url::image_resize_url;
use std::sync::Arc;

const KEY_PLAYER_VOLUME: &str = "player.volume";
const KEY_PLAYER_MODE: &str = "player.mode";
const KEY_PLAYER_QUEUE: &str = "player.queue";
const KEY_PLAYER_CURRENT_INDEX: &str = "player.current_index";
const KEY_PLAYER_POSITION_MS: &str = "player.position_ms";
const KEY_PLAYER_DURATION_MS: &str = "player.duration_ms";
const KEY_PLAYER_WAS_PLAYING: &str = "player.was_playing";
const KEY_WINDOW_CLOSE_BEHAVIOR: &str = "window.close_behavior";
const QR_POLL_INTERVAL: Duration = Duration::from_secs(2);
const QR_POLL_TIMEOUT: Duration = Duration::from_secs(120);
const PROGRESS_PERSIST_INTERVAL: Duration = Duration::from_secs(2);
const PLAYING_UI_NOTIFY_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataSource {
    Guest,
    User,
}

#[derive(Debug, Clone)]
struct DataState<T> {
    data: T,
    loading: bool,
    error: Option<String>,
    fetched_at_ms: Option<u64>,
    source: DataSource,
}

impl<T: Default> Default for DataState<T> {
    fn default() -> Self {
        Self {
            data: T::default(),
            loading: false,
            error: None,
            fetched_at_ms: None,
            source: DataSource::Guest,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedQueueItem {
    id: i64,
    name: String,
    artist: String,
    cover_url: Option<String>,
}

pub struct RootView {
    app: Entity<AppEntity>,
    player: Entity<PlayerEntity>,
    audio_bridge: Option<AudioBridgeEntity>,
    _audio_runtime: Option<AudioRuntimeHandle>,
    nav_search_input: Entity<input::InputState>,
    player_progress_slider: Entity<slider::SliderState>,
    player_volume_slider: Entity<slider::SliderState>,
    _subscriptions: Vec<Subscription>,
    search_error: Option<String>,
    search_state: DataState<Vec<search::SearchSong>>,
    main_scroll: SmoothScrollState,
    main_scroll_config: SmoothScrollConfig,
    settings_store: Option<SettingsStore>,
    state_store: Option<StateStore>,
    cache_store: Option<CacheIndexStore>,
    credential_store: CredentialStore,
    auth_bundle: AuthBundle,
    auth_account_summary: Option<String>,
    auth_user_name: Option<String>,
    auth_user_avatar: Option<String>,
    auth_user_id: Option<i64>,
    home_recommend_playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
    home_recommend_artists: DataState<Vec<library_actions::ArtistItem>>,
    home_new_albums: DataState<Vec<library_actions::AlbumItem>>,
    home_toplists: DataState<Vec<library_actions::ToplistItem>>,
    daily_tracks: DataState<Vec<library_actions::DailyTrackItem>>,
    personal_fm: DataState<Option<library_actions::FmTrackItem>>,
    discover_playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
    library_playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
    library_liked_tracks: DataState<Vec<library_actions::PlaylistTrackItem>>,
    library_liked_lyric_lines: Vec<String>,
    library_tab: library::LibraryTab,
    playlist_state: DataState<HashMap<i64, playlist::PlaylistPage>>,
    login_qr_key: Option<String>,
    login_qr_url: Option<String>,
    login_qr_image: Option<Arc<Image>>,
    login_qr_status: Option<String>,
    login_qr_polling: bool,
    login_qr_poll_started_at: Option<Instant>,
    login_qr_last_polled_at: Option<Instant>,
    last_progress_persist_at: Instant,
    last_progress_ui_notify_at: Instant,
    close_behavior: CloseBehavior,
    kernel_runtime: KernelRuntime,
}

impl RootView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut startup_error = None;
        let app_state = AppEntity::default();
        let mut player_state = PlayerEntity::default();
        player_state.mode = PlaybackMode::Sequence;

        let mut settings_store = None;
        let mut state_store = None;
        let mut cache_store = None;
        let mut persisted_was_playing = false;
        let mut close_behavior = CloseBehavior::default();

        if let Some(base_dir) = dirs::data_local_dir() {
            let db_path = base_dir.join("ame");
            match std::fs::create_dir_all(&db_path) {
                Ok(_) => match AppStorage::open(&db_path) {
                    Ok(storage) => {
                        match storage.settings() {
                            Ok(settings) => settings_store = Some(settings),
                            Err(err) => Self::push_error(
                                &mut startup_error,
                                format!("打开 settings 存储失败: {err}"),
                            ),
                        }
                        match storage.state() {
                            Ok(state) => state_store = Some(state),
                            Err(err) => Self::push_error(
                                &mut startup_error,
                                format!("打开 state 存储失败: {err}"),
                            ),
                        }
                        match storage.cache() {
                            Ok(cache) => cache_store = Some(cache),
                            Err(err) => Self::push_error(
                                &mut startup_error,
                                format!("打开 cache 存储失败: {err}"),
                            ),
                        }
                    }
                    Err(err) => {
                        Self::push_error(&mut startup_error, format!("打开存储失败: {err}"));
                    }
                },
                Err(err) => {
                    Self::push_error(&mut startup_error, format!("创建数据目录失败: {err}"))
                }
            }
        } else {
            Self::push_error(&mut startup_error, "无法定位系统数据目录".to_string());
        }

        if let Some(settings) = settings_store.as_ref() {
            match settings.get::<f32>(KEY_PLAYER_VOLUME) {
                Ok(Some(volume)) => player_state.set_volume(volume),
                Ok(None) => {}
                Err(err) => {
                    Self::push_error(&mut startup_error, format!("读取音量失败: {err}"));
                }
            }
            match settings.get::<PlaybackMode>(KEY_PLAYER_MODE) {
                Ok(Some(mode)) => player_state.mode = mode,
                Ok(None) => {}
                Err(err) => {
                    Self::push_error(&mut startup_error, format!("读取播放模式失败: {err}"));
                }
            }
            match settings.get::<CloseBehavior>(KEY_WINDOW_CLOSE_BEHAVIOR) {
                Ok(Some(value)) => close_behavior = value,
                Ok(None) => {}
                Err(err) => {
                    Self::push_error(&mut startup_error, format!("读取关闭行为失败: {err}"));
                }
            }
        }

        if let Some(state) = state_store.as_ref() {
            match state.get::<Vec<PersistedQueueItem>>(KEY_PLAYER_QUEUE) {
                Ok(Some(queue)) => {
                    player_state.set_queue(
                        queue
                            .into_iter()
                            .map(|item| QueueItem {
                                id: item.id,
                                name: item.name,
                                artist: item.artist,
                                cover_url: item.cover_url,
                                source_url: None,
                            })
                            .collect(),
                    );
                }
                Ok(None) => {}
                Err(err) => {
                    Self::push_error(&mut startup_error, format!("读取队列失败: {err}"));
                }
            }
            match state.get::<Option<usize>>(KEY_PLAYER_CURRENT_INDEX) {
                Ok(Some(index)) => {
                    player_state.current_index = index.filter(|i| *i < player_state.queue.len());
                }
                Ok(None) => {}
                Err(err) => {
                    Self::push_error(&mut startup_error, format!("读取当前索引失败: {err}"));
                }
            }
            match state.get::<u64>(KEY_PLAYER_POSITION_MS) {
                Ok(Some(position)) => player_state.position_ms = position,
                Ok(None) => {}
                Err(err) => {
                    Self::push_error(&mut startup_error, format!("读取播放进度失败: {err}"));
                }
            }
            match state.get::<u64>(KEY_PLAYER_DURATION_MS) {
                Ok(Some(duration)) => player_state.duration_ms = duration,
                Ok(None) => {}
                Err(err) => {
                    Self::push_error(&mut startup_error, format!("读取时长失败: {err}"));
                }
            }
            match state.get::<bool>(KEY_PLAYER_WAS_PLAYING) {
                Ok(Some(value)) => persisted_was_playing = value,
                Ok(None) => {}
                Err(err) => {
                    Self::push_error(&mut startup_error, format!("读取播放状态失败: {err}"));
                }
            }
        }

        player_state.is_playing = false;
        let initial_progress_ratio = player_state.progress_ratio();
        let initial_volume = player_state.volume;

        let app = cx.new(move |_| app_state.clone());
        let player = cx.new(move |_| player_state.clone());
        let nav_search_input = cx.new(|cx| {
            input::InputState::new(cx)
                .placeholder("搜索")
                .disabled(false)
        });
        let progress_slider_style = SliderStyle::for_variant(SliderVariant::ProgressLine)
            .thumb_visibility(SliderThumbVisibility::DragOnly)
            .root_height(nekowg::px(2.))
            .track_height(nekowg::px(2.));
        let volume_slider_style = SliderStyle::for_variant(SliderVariant::Default)
            .thumb_visibility(SliderThumbVisibility::HoverOrDrag);
        let player_progress_slider = cx.new(|cx| {
            slider::SliderState::new(cx)
                .range(0.0, 1.0)
                .value(initial_progress_ratio)
                .style(progress_slider_style)
        });
        let player_volume_slider = cx.new(|cx| {
            slider::SliderState::new(cx)
                .range(0.0, 1.0)
                .value(initial_volume)
                .style(volume_slider_style)
        });
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &nav_search_input,
            |this, _, event: &input::InputEvent, cx| match event {
                input::InputEvent::Change(text) => {
                    this.app
                        .update(cx, |app, _| app.set_search_query(text.to_string()));
                }
                input::InputEvent::Submit(text) => {
                    this.app
                        .update(cx, |app, _| app.set_search_query(text.to_string()));
                    this.queue_kernel_command(AppCommand::SubmitSearchFromQuery);
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &player_volume_slider,
            |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = *event {
                    this.set_volume_absolute(value, cx);
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &player_progress_slider,
            |this, _, event: &SliderEvent, cx| match *event {
                SliderEvent::Change(value) => this.preview_seek_ratio(value, cx),
                SliderEvent::Commit(value) => this.commit_seek_ratio(value, cx),
            },
        ));

        let credential_store = CredentialStore;
        let mut auth_bundle = AuthBundle::default();
        match credential_store.load_auth_bundle() {
            Ok(Some(bundle)) => auth_bundle = bundle,
            Ok(None) => {}
            Err(err) => {
                Self::push_error(&mut startup_error, format!("读取 keyring 凭据失败: {err}"));
            }
        }

        let (audio_bridge, audio_runtime, search_error) =
            match AudioService::spawn(AudioConfig::default()) {
                Ok((service, runtime)) => {
                    (Some(AudioBridgeEntity::new(service)), Some(runtime), None)
                }
                Err(err) => (None, None, Some(format!("音频初始化失败: {err}"))),
            };

        let main_scroll_config = SmoothScrollConfig::default();
        let tick_ms = main_scroll_config.tick_ms;
        let kernel_runtime = KernelRuntime::start();
        let mut root = Self {
            app,
            player,
            audio_bridge,
            _audio_runtime: audio_runtime,
            nav_search_input,
            player_progress_slider,
            player_volume_slider,
            _subscriptions: subscriptions,
            search_error: None,
            search_state: DataState::default(),
            main_scroll: SmoothScrollState::new(nekowg::ScrollHandle::default()),
            main_scroll_config,
            settings_store,
            state_store,
            cache_store,
            credential_store,
            auth_bundle,
            auth_account_summary: None,
            auth_user_name: None,
            auth_user_avatar: None,
            auth_user_id: None,
            home_recommend_playlists: DataState::default(),
            home_recommend_artists: DataState::default(),
            home_new_albums: DataState::default(),
            home_toplists: DataState::default(),
            daily_tracks: DataState::default(),
            personal_fm: DataState::default(),
            discover_playlists: DataState::default(),
            library_playlists: DataState::default(),
            library_liked_tracks: DataState::default(),
            library_liked_lyric_lines: Vec::new(),
            library_tab: library::LibraryTab::Created,
            playlist_state: DataState {
                data: HashMap::new(),
                ..DataState::default()
            },
            login_qr_key: None,
            login_qr_url: None,
            login_qr_image: None,
            login_qr_status: None,
            login_qr_polling: false,
            login_qr_poll_started_at: None,
            login_qr_last_polled_at: None,
            last_progress_persist_at: Instant::now(),
            last_progress_ui_notify_at: Instant::now(),
            close_behavior,
            kernel_runtime,
        };
        root.main_scroll.target_y = nekowg::px(0.);

        if let Some(err) = search_error {
            Self::push_error(&mut root.search_error, err);
        }
        if let Some(err) = startup_error {
            Self::push_error(&mut root.search_error, err);
        }

        if let Some(audio) = &root.audio_bridge {
            let _ = audio.send(ame_audio::AudioCommand::SetVolume(
                root.player.read(cx).volume,
            ));
        }

        root.persist_player_settings(cx);
        root.persist_player_runtime(cx);

        if root.auth_bundle.music_u.is_none() && root.auth_bundle.music_a.is_none() {
            root.ensure_guest_session();
        } else {
            root.refresh_login_summary();
        }
        root.refresh_home_data();
        root.refresh_discover_playlists();
        if persisted_was_playing {
            root.login_qr_status = root
                .login_qr_status
                .or_else(|| Some("已恢复上次播放状态（未自动播放）".to_string()));
        }

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(tick_ms))
                    .await;
                let updated = this.update(cx, |this, cx| {
                    let now = Instant::now();
                    let mut should_notify = false;
                    if this.drain_kernel_events(cx) {
                        should_notify = true;
                    }
                    this.sync_audio_bridge(cx);
                    if this.main_scroll.tick(&this.main_scroll_config) {
                        should_notify = true;
                    }
                    if this.tick_qr_poll(now) {
                        should_notify = true;
                    }
                    if this.player.read(cx).is_playing
                        && now.duration_since(this.last_progress_ui_notify_at)
                            >= PLAYING_UI_NOTIFY_INTERVAL
                    {
                        this.last_progress_ui_notify_at = now;
                        should_notify = true;
                    }
                    this.persist_progress_by_interval(now, cx);
                    if should_notify {
                        cx.notify();
                    }
                });
                if updated.is_err() {
                    break;
                }
            }
        })
        .detach();

        root
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root_entity = cx.entity();
        let pathname = router::current_path(cx).as_ref().to_string();

        let _app = self.app.read(cx).clone();
        let player = self.player.read(cx).clone();
        let player_entity = self.player.clone();
        let login_model = self.login_view_model();
        let close_root = root_entity.clone();
        let top = title_bar::render(
            &TitleBarModel {
                title: "Ame".into(),
                is_maximized: window.is_maximized(),
            },
            &TitleBarActions {
                on_min: Arc::new(|window, _| window.minimize_window()),
                on_toggle_max_restore: Arc::new(|window, _| window.zoom_window()),
                on_close: Arc::new(move |window, cx| {
                    close_root.update(cx, |this, cx| this.request_window_close(window, cx));
                }),
            },
        );

        let nav_avatar = self
            .auth_user_avatar
            .as_ref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| image_resize_url(value, "64y64"))
            .unwrap_or_else(|| "image/akkarin.webp".to_string());

        let nav = nav_bar::render(
            &NavBarModel {
                pathname: pathname.clone().into(),
                search_input: self.nav_search_input.clone(),
                avatar_asset: nav_avatar.into(),
            },
            &NavBarActions {
                on_back: Arc::new(|_| {}),
                on_forward: Arc::new(|_| {}),
                on_home: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::Navigate("/".to_string()))
                        });
                    })
                },
                on_discover: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::Navigate("/explore".to_string()))
                        });
                    })
                },
                on_library: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::Navigate("/library".to_string()))
                        });
                    })
                },
                on_profile: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::Navigate("/login".to_string()))
                        });
                    })
                },
            },
        );
        let routes = self.render_routes(
            cx,
            root_entity,
            player_entity,
            routes::RoutesModel {
                home_recommend_playlists: self.home_recommend_playlists.clone(),
                home_recommend_artists: self.home_recommend_artists.clone(),
                home_new_albums: self.home_new_albums.clone(),
                home_toplists: self.home_toplists.clone(),
                daily_tracks: self.daily_tracks.clone(),
                personal_fm: self.personal_fm.clone(),
                is_user_logged_in: self
                    .auth_bundle
                    .music_u
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty()),
                discover_playlists: self.discover_playlists.clone(),
                search_state: self.search_state.clone(),
                library_playlists: self.library_playlists.clone(),
                library_liked_tracks: self.library_liked_tracks.clone(),
                library_liked_lyric_lines: self.library_liked_lyric_lines.clone(),
                library_tab: self.library_tab,
                playlist_state: self.playlist_state.clone(),
                page_scroll_handle: self.main_scroll.handle.clone(),
                auth_account_summary: self.auth_account_summary.clone(),
                auth_user_name: self.auth_user_name.clone(),
                auth_user_avatar: self.auth_user_avatar.clone(),
                login_model,
                close_behavior_label: self.close_behavior.label().to_string(),
            },
        );
        let (current_song, current_artist, current_cover_url) = player
            .current_item()
            .map(|item| {
                (
                    item.name.clone(),
                    item.artist.clone(),
                    item.cover_url.clone(),
                )
            })
            .unwrap_or_else(|| ("未播放".to_string(), "未知作家".to_string(), None));
        let bottom = bottom_bar::render(
            &bottom_bar::BottomBarModel {
                current_song: current_song.into(),
                current_artist: current_artist.into(),
                current_cover_url: current_cover_url.map(Into::into),
                is_playing: player.is_playing,
                mode: player.mode,
                volume: player.volume,
                progress_slider: self.player_progress_slider.clone(),
                volume_slider: self.player_volume_slider.clone(),
            },
            &bottom_bar::BottomBarActions {
                on_prev: {
                    let root_entity = cx.entity();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::PreviousTrack)
                        });
                    })
                },
                on_toggle: {
                    let root_entity = cx.entity();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::TogglePlay)
                        });
                    })
                },
                on_next: {
                    let root_entity = cx.entity();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::NextTrack)
                        });
                    })
                },
                on_open_queue: {
                    let root_entity = cx.entity();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::Navigate("/next".to_string()))
                        });
                    })
                },
                on_cycle_mode: {
                    let root_entity = cx.entity();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, _| {
                            this.queue_kernel_command(AppCommand::CyclePlayMode)
                        });
                    })
                },
            },
        );

        let main_content = div()
            .id("main-content")
            .w_full()
            .flex_grow()
            .min_h_0()
            .relative()
            .overflow_hidden()
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                this.main_scroll.apply_scroll_delta(
                    event.delta,
                    window.line_height(),
                    &this.main_scroll_config,
                );
                cx.stop_propagation();
                cx.notify();
            }))
            .px(relative(0.1))
            .py_0()
            .child(
                div()
                    .id("main-scroll-viewport")
                    .w_full()
                    .h_full()
                    .track_scroll(&self.main_scroll.handle)
                    .overflow_hidden()
                    .child(routes),
            )
            .child(self.render_scrollbar(cx))
            .into_any_element();

        div()
            .size_full()
            .bg(rgb(theme::COLOR_BODY_BG_DARK))
            .text_color(rgb(theme::COLOR_TEXT_DARK))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(top)
            .child(nav)
            .child(main_content)
            .child(bottom)
    }
}

impl RootView {
    fn render_scrollbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(metrics) = self.main_scroll.thumb_metrics(&self.main_scroll_config) else {
            return div().into_any_element();
        };
        let opacity = self
            .main_scroll
            .scrollbar_opacity(&self.main_scroll_config, std::time::Instant::now());
        let visible = opacity > 0.001;
        let viewport_origin = self.main_scroll.handle.bounds().origin;

        crate::component::scroll::render_scrollbar_overlay(
            &ScrollBarModel {
                metrics,
                opacity,
                visible,
                dragging: self.main_scroll.dragging,
                hovering_bar: self.main_scroll.hovering_bar,
                viewport_origin,
                style: ScrollBarStyle::default()
                    .overlay_width(nekowg::px(self.main_scroll_config.overlay_width_px)),
            },
            &ScrollBarActions::<Self> {
                on_hover: Arc::new(move |this, hovered, cx| {
                    this.main_scroll.set_hovering(hovered);
                    cx.notify();
                }),
                on_mouse_down: Arc::new(move |this, local_position, cx| {
                    if this
                        .main_scroll
                        .begin_drag_or_jump(local_position, &this.main_scroll_config)
                    {
                        cx.notify();
                    }
                }),
                on_mouse_move: Arc::new(move |this, local_position, cx| {
                    if this
                        .main_scroll
                        .drag_to(local_position, &this.main_scroll_config)
                    {
                        cx.notify();
                    }
                }),
                on_mouse_up: Arc::new(move |this, cx| {
                    if this.main_scroll.end_drag() {
                        cx.notify();
                    }
                }),
            },
            cx,
        )
    }

    fn push_error(slot: &mut Option<String>, message: String) {
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

    fn login_view_model(&self) -> login::LoginViewModel {
        let auth_state = if self.auth_bundle.music_u.is_some() {
            "账号登录"
        } else if self.auth_bundle.music_a.is_some() {
            "游客登录"
        } else {
            "未登录"
        };

        login::LoginViewModel {
            auth_state: auth_state.to_string(),
            account_summary: self.auth_account_summary.clone(),
            qr_status: self.login_qr_status.clone(),
            qr_url: self.login_qr_url.clone(),
            qr_image: self.login_qr_image.clone(),
            polling: self.login_qr_polling,
            error: self.search_error.clone(),
        }
    }
}




