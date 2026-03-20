mod actions;
mod lifecycle;
mod render;

use std::time::Instant;

use ame_audio::AudioRuntimeHandle;
use nekowg::{Context, Entity, Subscription, Window, prelude::*};

use crate::app::env::AppEnv;
use crate::app::page_host::PageHostView;
use crate::app::runtime::{AppRuntime, RuntimeBootstrap};
use crate::component::{
    input,
    scroll::{SmoothScrollConfig, SmoothScrollState},
    slider::{self, SliderStyle, SliderThumbVisibility, SliderVariant},
};

pub struct RootView {
    env: AppEnv,
    runtime: AppRuntime,
    page_host: Entity<PageHostView>,
    nav_search_input: Entity<input::InputState>,
    player_progress_slider: Entity<slider::SliderState>,
    player_volume_slider: Entity<slider::SliderState>,
    _subscriptions: Vec<Subscription>,
    main_scroll: SmoothScrollState,
    main_scroll_config: SmoothScrollConfig,
    last_progress_ui_notify_at: Instant,
    last_player_progress_persist_at: Instant,
    _audio_runtime: Option<AudioRuntimeHandle>,
}

impl RootView {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let RuntimeBootstrap {
            runtime,
            env,
            audio_runtime,
        } = AppRuntime::bootstrap(cx);
        let initial_progress_ratio = env.player().read(cx).progress_ratio();
        let initial_volume = env.player().read(cx).volume;

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

        let main_scroll_config = SmoothScrollConfig::default();
        let tick_ms = main_scroll_config.tick_ms;
        let main_scroll = SmoothScrollState::new(nekowg::ScrollHandle::default());
        let page_scroll_handle = main_scroll.handle.clone();
        let page_host = {
            let runtime = runtime.clone();
            let page_scroll_handle = page_scroll_handle.clone();
            cx.new(move |cx| PageHostView::new(runtime.clone(), page_scroll_handle.clone(), cx))
        };

        let mut root = Self {
            env,
            runtime,
            page_host,
            nav_search_input,
            player_progress_slider,
            player_volume_slider,
            _subscriptions: Vec::new(),
            main_scroll,
            main_scroll_config,
            last_progress_ui_notify_at: Instant::now(),
            last_player_progress_persist_at: Instant::now(),
            _audio_runtime: audio_runtime,
        };
        root.main_scroll.target_y = nekowg::px(0.);
        root.setup_subscriptions(cx);
        root.sync_player_controls(cx);
        root.sync_route(cx);
        root.prime_session(cx);
        root.spawn_runtime_tick(tick_ms, cx);
        root
    }
}
