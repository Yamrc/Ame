mod routes;

use crate::router;
use nekowg::{
    AnyElement, Context, Entity, Render, ScrollWheelEvent, Subscription, Window, div, prelude::*,
    relative, rgb,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ame_audio::{AudioConfig, AudioService};

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
use crate::entity::audio_bridge::AudioBridgeEntity;
use crate::entity::player_controller::PlayerController;
use crate::entity::runtime::{AppRuntime, RuntimeBootstrap};
use crate::entity::services::auth;
use crate::entity::session_controller::SessionController;
use crate::util::url::image_resize_url;
use crate::view::{daily_tracks, discover, home, library, login, next, playlist, search, settings};

const PLAYING_UI_NOTIFY_INTERVAL: Duration = Duration::from_millis(100);

pub struct RootView {
    runtime: AppRuntime,
    player_controller: Entity<PlayerController>,
    session_controller: Entity<SessionController>,
    pages: routes::RootPages,
    nav_search_input: Entity<input::InputState>,
    player_progress_slider: Entity<slider::SliderState>,
    player_volume_slider: Entity<slider::SliderState>,
    _subscriptions: Vec<Subscription>,
    main_scroll: SmoothScrollState,
    main_scroll_config: SmoothScrollConfig,
    last_progress_ui_notify_at: Instant,
}

impl RootView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let RuntimeBootstrap { runtime } = AppRuntime::bootstrap(cx);
        let session = runtime.session.read(cx).clone();
        let initial_progress_ratio = runtime.player.read(cx).progress_ratio();
        let initial_volume = runtime.player.read(cx).volume;

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

        let (audio_bridge, audio_runtime, audio_error) =
            match AudioService::spawn(AudioConfig::default()) {
                Ok((service, runtime)) => {
                    (Some(AudioBridgeEntity::new(service)), Some(runtime), None)
                }
                Err(err) => (None, None, Some(format!("音频初始化失败: {err}"))),
            };

        let main_scroll_config = SmoothScrollConfig::default();
        let tick_ms = main_scroll_config.tick_ms;
        let main_scroll = SmoothScrollState::new(nekowg::ScrollHandle::default());
        let page_scroll_handle = main_scroll.handle.clone();

        let player_controller = {
            let runtime = runtime.clone();
            cx.new(move |cx| {
                PlayerController::new(runtime.clone(), audio_bridge, audio_runtime, cx)
            })
        };
        let session_controller = {
            let runtime = runtime.clone();
            cx.new(move |cx| SessionController::new(runtime.clone(), cx))
        };

        let pages = routes::RootPages {
            home: {
                let runtime = runtime.clone();
                let player_controller = player_controller.clone();
                cx.new(move |cx| {
                    home::HomePageView::new(runtime.clone(), player_controller.clone(), cx)
                })
            },
            discover: {
                let runtime = runtime.clone();
                cx.new(move |cx| discover::DiscoverPageView::new(runtime.clone(), cx))
            },
            library: {
                let runtime = runtime.clone();
                let player_controller = player_controller.clone();
                cx.new(move |cx| {
                    library::LibraryPageView::new(runtime.clone(), player_controller.clone(), cx)
                })
            },
            search: {
                let runtime = runtime.clone();
                let player_controller = player_controller.clone();
                cx.new(move |cx| {
                    search::SearchPageView::new(runtime.clone(), player_controller.clone(), cx)
                })
            },
            daily_tracks: {
                let runtime = runtime.clone();
                let player_controller = player_controller.clone();
                cx.new(move |cx| {
                    daily_tracks::DailyTracksPageView::new(
                        runtime.clone(),
                        player_controller.clone(),
                        cx,
                    )
                })
            },
            next: {
                let runtime = runtime.clone();
                let player_controller = player_controller.clone();
                let page_scroll_handle = page_scroll_handle.clone();
                cx.new(move |cx| {
                    next::NextPageView::new(
                        runtime.clone(),
                        player_controller.clone(),
                        page_scroll_handle.clone(),
                        cx,
                    )
                })
            },
            playlist: {
                let runtime = runtime.clone();
                let player_controller = player_controller.clone();
                let page_scroll_handle = page_scroll_handle.clone();
                cx.new(move |cx| {
                    playlist::PlaylistPageView::new(
                        runtime.clone(),
                        player_controller.clone(),
                        page_scroll_handle.clone(),
                        cx,
                    )
                })
            },
            settings: {
                let runtime = runtime.clone();
                cx.new(move |cx| settings::SettingsPageView::new(runtime.clone(), cx))
            },
            login: {
                let runtime = runtime.clone();
                let session_controller = session_controller.clone();
                cx.new(move |cx| {
                    login::LoginPageView::new(runtime.clone(), session_controller.clone(), cx)
                })
            },
        };

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &nav_search_input,
            |this, _, event: &input::InputEvent, cx| match event {
                input::InputEvent::Change(text) => {
                    this.runtime
                        .app
                        .update(cx, |app, _| app.set_search_query(text.to_string()));
                }
                input::InputEvent::Submit(text) => {
                    let text = text.to_string();
                    this.runtime
                        .app
                        .update(cx, |app, _| app.set_search_query(text.clone()));
                    let query = text.trim().to_string();
                    if query.is_empty() {
                        router::navigate(cx, "/search");
                    } else {
                        let sanitized = query.replace('/', " ");
                        router::navigate(cx, format!("/search/{sanitized}"));
                    }
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &player_volume_slider,
            |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = *event {
                    this.player_controller
                        .update(cx, |player, cx| player.set_volume_absolute(value, cx));
                }
            },
        ));
        subscriptions.push(cx.subscribe(
            &player_progress_slider,
            |this, _, event: &SliderEvent, cx| {
                match *event {
                    SliderEvent::Change(value) => this
                        .player_controller
                        .update(cx, |player, cx| player.preview_seek_ratio(value, cx)),
                    SliderEvent::Commit(value) => this
                        .player_controller
                        .update(cx, |player, cx| player.commit_seek_ratio(value, cx)),
                }
            },
        ));

        let mut root = Self {
            runtime,
            player_controller,
            session_controller,
            pages,
            nav_search_input,
            player_progress_slider,
            player_volume_slider,
            _subscriptions: subscriptions,
            main_scroll,
            main_scroll_config,
            last_progress_ui_notify_at: Instant::now(),
        };
        root.main_scroll.target_y = nekowg::px(0.);
        root.sync_player_controls(cx);

        if let Some(err) = audio_error {
            auth::push_shell_error(&root.runtime, err, cx);
        }

        if session.auth_bundle.music_u.is_none() && session.auth_bundle.music_a.is_none() {
            root.session_controller
                .update(cx, |controller, cx| controller.ensure_guest_session(cx));
        } else {
            root.session_controller
                .update(cx, |controller, cx| controller.refresh_login_summary(cx));
        }

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(tick_ms))
                    .await;
                let updated = this.update(cx, |this, cx| {
                    let now = Instant::now();
                    let mut should_notify = false;

                    this.player_controller.update(cx, |player, cx| {
                        player.sync_audio_bridge(cx);
                        player.persist_progress_by_interval(now, cx);
                    });
                    this.session_controller.update(cx, |session, cx| {
                        let _ = session.tick_qr_poll(now, cx);
                    });
                    if this.main_scroll.tick(&this.main_scroll_config) {
                        should_notify = true;
                    }
                    this.sync_player_controls(cx);
                    if this.runtime.player.read(cx).is_playing
                        && now.duration_since(this.last_progress_ui_notify_at)
                            >= PLAYING_UI_NOTIFY_INTERVAL
                    {
                        this.last_progress_ui_notify_at = now;
                        should_notify = true;
                    }
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

    fn sync_player_controls(&mut self, cx: &mut Context<Self>) {
        let player_snapshot = self.runtime.player.read(cx).clone();
        self.player_volume_slider.update(cx, |slider, _| {
            if !slider.is_dragging() {
                slider.set_value_silent(player_snapshot.volume);
            }
        });
        self.player_progress_slider.update(cx, |slider, _| {
            if !slider.is_dragging() {
                slider.set_value_silent(player_snapshot.progress_ratio());
            }
        });
    }

    pub(crate) fn navigate_to(
        &mut self,
        path: impl Into<nekowg::SharedString>,
        cx: &mut Context<Self>,
    ) {
        router::navigate(cx, path.into());
        cx.notify();
    }

    pub(crate) fn request_window_close(
        &mut self,
        window: &mut nekowg::Window,
        cx: &mut Context<Self>,
    ) {
        match self.runtime.shell.read(cx).close_behavior {
            crate::entity::app::CloseBehavior::HideToTray => {
                window.hide();
            }
            crate::entity::app::CloseBehavior::Exit => {
                self.prepare_app_exit(cx);
                cx.quit();
            }
            crate::entity::app::CloseBehavior::Ask => {
                let window_handle = window.window_handle();
                let answer = window.prompt(
                    nekowg::PromptLevel::Info,
                    "确定要关闭吗？",
                    Some("以下选择会作为默认行为，可以在设置中修改"),
                    &[
                        nekowg::PromptButton::new("隐藏到托盘"),
                        nekowg::PromptButton::ok("退出应用"),
                        nekowg::PromptButton::cancel("取消"),
                    ],
                    cx,
                );
                let root = cx.entity();
                cx.spawn(async move |_, cx| {
                    let Ok(choice) = answer.await else {
                        return;
                    };
                    root.update(cx, |this, cx| match choice {
                        0 => {
                            crate::entity::services::shell::set_close_behavior(
                                &this.runtime,
                                crate::entity::app::CloseBehavior::HideToTray,
                                cx,
                            );
                            let _ = window_handle.update(cx, |_, window, _cx| {
                                window.hide();
                            });
                        }
                        1 => {
                            crate::entity::services::shell::set_close_behavior(
                                &this.runtime,
                                crate::entity::app::CloseBehavior::Exit,
                                cx,
                            );
                            this.prepare_app_exit(cx);
                            cx.quit();
                        }
                        _ => {}
                    });
                })
                .detach();
            }
        }
    }

    pub(crate) fn prepare_app_exit(&mut self, cx: &mut Context<Self>) {
        self.session_controller
            .update(cx, |session, cx| session.stop_background_work(cx));
        self.player_controller
            .update(cx, |player, cx| player.prepare_app_exit(cx));
    }

    pub(crate) fn tray_toggle_playback(&mut self, cx: &mut Context<Self>) {
        self.player_controller
            .update(cx, |player, cx| player.toggle_playback(cx));
    }

    pub(crate) fn tray_next(&mut self, cx: &mut Context<Self>) {
        self.player_controller
            .update(cx, |player, cx| player.play_next(cx));
    }

    pub(crate) fn tray_previous(&mut self, cx: &mut Context<Self>) {
        self.player_controller
            .update(cx, |player, cx| player.play_previous(cx));
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root_entity = cx.entity();
        let pathname = router::current_path(cx).as_ref().to_string();
        let player = self.runtime.player.read(cx).clone();
        let session = self.runtime.session.read(cx).clone();
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

        let nav_avatar = session
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
                        root_entity.update(cx, |this, cx| this.navigate_to("/", cx));
                    })
                },
                on_discover: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.navigate_to("/explore", cx));
                    })
                },
                on_library: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.navigate_to("/library", cx));
                    })
                },
                on_profile: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.navigate_to("/login", cx));
                    })
                },
            },
        );
        let routes = self.render_routes(cx);
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
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.tray_previous(cx));
                    })
                },
                on_toggle: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.tray_toggle_playback(cx));
                    })
                },
                on_next: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.tray_next(cx));
                    })
                },
                on_open_queue: {
                    let root_entity = root_entity.clone();
                    Arc::new(move |cx| {
                        root_entity.update(cx, |this, cx| this.navigate_to("/next", cx));
                    })
                },
                on_cycle_mode: {
                    let player_controller = self.player_controller.clone();
                    Arc::new(move |cx| {
                        player_controller.update(cx, |player, cx| player.cycle_play_mode(cx));
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
            .font_family("Noto Sans JP")
            .font_family("Noto Sans SC")
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
}
