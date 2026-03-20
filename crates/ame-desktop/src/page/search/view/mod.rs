mod load;

use nekowg::{
    App, Context, Entity, FontWeight, Render, Subscription, Window, div, prelude::*, px, rgb,
};
use std::rc::Rc;
use std::sync::Arc;

use crate::app::page::{PageLifecycle, PageRetentionPolicy};
use crate::app::route::AppRoute;
use crate::app::router;
use crate::app::runtime::AppRuntime;
use crate::component::{page, theme};
use crate::domain::player;

use super::sections::{
    EnqueueSongHandler, PlaySongHandler, PlaylistOpenHandler, SearchTypeNavigateHandler,
    render_overview_sections, render_type_page,
};
use super::state::SearchPageState;
use super::types::SearchPageRoute;

pub(super) const TYPE_PAGE_LIMIT: u32 = 30;
pub(super) type SessionLoadKey = (Option<i64>, bool);

pub struct SearchPageView {
    runtime: AppRuntime,
    route: SearchPageRoute,
    state: Entity<SearchPageState>,
    last_session_key: SessionLoadKey,
    active: bool,
    _subscriptions: Vec<Subscription>,
}

impl SearchPageView {
    pub fn new(runtime: AppRuntime, route: SearchPageRoute, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| SearchPageState::default());
        let last_session_key = load::session_load_key(&runtime, cx);
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&state, |_, _, cx| {
            cx.notify();
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        subscriptions.push(cx.observe(&runtime.player, |_, _, cx| {
            cx.notify();
        }));
        Self {
            runtime,
            route,
            state,
            last_session_key,
            active: false,
            _subscriptions: subscriptions,
        }
    }
}

impl Render for SearchPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let route = &self.route;
        let page = cx.entity();
        let search_state = self.state.read(cx);
        let current_playing_track_id = self
            .runtime
            .player
            .read(cx)
            .current_item()
            .map(|item| item.id);
        let on_play_song: PlaySongHandler = {
            let page = page.clone();
            Arc::new(move |song, cx| {
                let input = player::QueueTrackInput::from(song);
                page.update(cx, |this, cx| {
                    player::enqueue_track(&this.runtime, input, true, cx);
                });
            })
        };
        let on_enqueue_song: EnqueueSongHandler = {
            let page = page.clone();
            Arc::new(move |song, cx| {
                let input = player::QueueTrackInput::from(song);
                page.update(cx, |this, cx| {
                    player::enqueue_track(&this.runtime, input, false, cx);
                });
            })
        };
        let on_open_playlist: PlaylistOpenHandler = {
            let page = page.clone();
            Rc::new(move |playlist_id, cx| {
                page.update(cx, |_, cx| {
                    router::navigate_route(cx, AppRoute::Playlist { id: playlist_id });
                });
            })
        };
        let keyword = route.keyword.clone();
        let on_navigate_type: SearchTypeNavigateHandler = {
            let page = page.clone();
            Rc::new(move |route_type, cx| {
                let keyword = keyword.clone();
                page.update(cx, |_, cx| {
                    if keyword.trim().is_empty() {
                        router::navigate_route(cx, AppRoute::Search);
                        return;
                    }
                    router::navigate_route(
                        cx,
                        AppRoute::SearchCollection {
                            query: keyword.clone(),
                            kind: route_type.as_kind(),
                        },
                    );
                });
            })
        };

        let title = match route.route_type {
            Some(route_type) if !route.keyword.is_empty() => Some(
                div()
                    .text_size(px(30.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                    .child(format!("搜索{} \"{}\"", route_type.label(), route.keyword))
                    .into_any_element(),
            ),
            _ => None,
        };

        let content = match route.route_type {
            None => {
                let status = page::status_banner(
                    search_state.overview.loading,
                    search_state.overview.error.as_deref(),
                    "搜索中...",
                    "搜索失败",
                );
                let body = if route.keyword.is_empty() {
                    page::empty_card("输入关键字搜索")
                } else if !search_state.overview.data.has_result()
                    && !search_state.overview.loading
                    && search_state.overview.error.is_none()
                {
                    page::empty_card("暂无结果")
                } else {
                    render_overview_sections(
                        &search_state.overview.data,
                        on_play_song,
                        on_enqueue_song,
                        on_open_playlist,
                        on_navigate_type,
                    )
                };
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_5()
                    .child(status)
                    .child(body)
                    .into_any_element()
            }
            Some(route_type) => render_type_page(
                route_type,
                search_state,
                current_playing_track_id,
                on_play_song,
                on_enqueue_song,
                on_open_playlist,
                {
                    let page = cx.entity();
                    Rc::new(move |cx: &mut App| {
                        page.update(cx, |this, cx| this.load_more(cx));
                    })
                },
            ),
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(28.))
            .gap_5()
            .children(title)
            .child(content)
    }
}

impl PageLifecycle for SearchPageView {
    fn on_activate(&mut self, cx: &mut Context<Self>) {
        self.active = true;
        self.ensure_loaded(cx);
    }

    fn snapshot_policy(&self) -> PageRetentionPolicy {
        PageRetentionPolicy::SnapshotOnly
    }

    fn release_view_resources(&mut self, cx: &mut Context<Self>) {
        self.active = false;
        self.release_search_heavy_data(cx);
    }
}
