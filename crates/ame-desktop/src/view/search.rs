use std::sync::Arc;

use nekowg::{
    AnyElement, App, Context, Entity, FontWeight, Render, Subscription, Window, div, prelude::*,
    px, rgb,
};

use crate::component::theme;
use crate::component::track_item::{self, TrackItemActions, TrackItemProps};
use crate::entity::pages::DataState;
use crate::entity::player_controller::PlayerController;
use crate::entity::runtime::AppRuntime;
use crate::entity::services::pages;
use crate::router::{self, RouterState, use_params};
use crate::view::common;

type EnqueueSongHandler = Arc<dyn Fn(SearchSong, &mut App)>;

#[derive(Debug, Clone)]
pub struct SearchSong {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SearchPageSnapshot {
    pub keyword: String,
    pub loading: bool,
    pub error: Option<String>,
    pub results: Vec<SearchSong>,
    pub current_playing_track_id: Option<i64>,
}

impl SearchPageSnapshot {
    pub fn from_state(
        keyword: impl Into<String>,
        state: &DataState<Vec<SearchSong>>,
        current_playing_track_id: Option<i64>,
    ) -> Self {
        Self {
            keyword: keyword.into(),
            loading: state.loading,
            error: state.error.clone(),
            results: state.data.clone(),
            current_playing_track_id,
        }
    }
}

pub fn render_row(
    song: SearchSong,
    is_playing: bool,
    on_enqueue: impl Fn(&mut App) + 'static,
) -> AnyElement {
    track_item::render(
        TrackItemProps {
            id: song.id,
            title: song.name,
            alias: song.alias,
            artists: song.artists,
            album: song.album,
            duration_ms: song.duration_ms,
            cover_url: None,
            show_cover: false,
            is_playing,
        },
        TrackItemActions {
            on_enqueue: Some(std::sync::Arc::new(on_enqueue)),
            ..TrackItemActions::default()
        },
    )
}

pub struct SearchPageView {
    runtime: AppRuntime,
    player_controller: Entity<PlayerController>,
    last_keyword: String,
    _subscriptions: Vec<Subscription>,
}

impl SearchPageView {
    pub fn new(
        runtime: AppRuntime,
        player_controller: Entity<PlayerController>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe_global::<RouterState>(|this, cx| {
            this.sync_route_query(cx);
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            if this.is_active(cx) {
                this.last_keyword.clear();
                this.sync_route_query(cx);
            }
        }));
        let mut this = Self {
            runtime,
            player_controller,
            last_keyword: String::new(),
            _subscriptions: subscriptions,
        };
        this.sync_route_query(cx);
        this
    }

    fn is_active(&self, cx: &mut Context<Self>) -> bool {
        router::current_path(cx).as_ref().starts_with("/search")
    }

    fn current_keyword(&self, cx: &mut Context<Self>) -> String {
        if !self.is_active(cx) {
            return String::new();
        }
        use_params(cx)
            .get("keywords")
            .map(|value| value.as_ref().to_string())
            .unwrap_or_default()
    }

    fn sync_route_query(&mut self, cx: &mut Context<Self>) {
        if !self.is_active(cx) {
            return;
        }
        let keyword = self.current_keyword(cx);
        if keyword == self.last_keyword {
            return;
        }
        if keyword.trim().is_empty() {
            self.last_keyword.clear();
            self.runtime.search.update(cx, |search, cx| {
                search.results.clear();
                cx.notify();
            });
            return;
        }
        self.last_keyword = keyword.clone();
        let session = self.runtime.session.read(cx).clone();
        let cookie = crate::action::auth_actions::build_cookie_header(&session.auth_bundle);
        self.runtime.search.update(cx, |search, cx| {
            search
                .results
                .begin(crate::entity::pages::DataSource::Guest);
            cx.notify();
        });

        let page = cx.entity();
        let request_keyword = keyword.clone();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(
                    async move { pages::fetch_search_payload(&request_keyword, cookie.as_deref()) },
                )
                .await;
            page.update(cx, |this, cx| this.apply_search_result(keyword, result, cx));
        })
        .detach();
    }

    fn apply_search_result(
        &mut self,
        keyword: String,
        result: Result<Vec<SearchSong>, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.is_active(cx) || self.current_keyword(cx) != keyword {
            return;
        }

        self.runtime.search.update(cx, |search, cx| {
            match result {
                Ok(items) => search.results.succeed(items, None),
                Err(err) => {
                    search.results.clear();
                    search.results.fail(err);
                }
            }
            cx.notify();
        });
    }
}

impl Render for SearchPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let keyword = self.current_keyword(cx);
        let snapshot = SearchPageSnapshot::from_state(
            keyword,
            &self.runtime.search.read(cx).results,
            self.runtime
                .player
                .read(cx)
                .current_item()
                .map(|item| item.id),
        );
        let on_enqueue: EnqueueSongHandler = {
            let player_controller = self.player_controller.clone();
            Arc::new(move |song, cx| {
                player_controller.update(cx, |this, cx| this.enqueue_track(song.into(), true, cx));
            })
        };
        let rows = snapshot
            .results
            .into_iter()
            .map(|song| {
                let is_playing = snapshot.current_playing_track_id == Some(song.id);
                let on_enqueue = on_enqueue.clone();
                let song_for_click = song.clone();
                render_row(song, is_playing, move |cx| {
                    on_enqueue(song_for_click.clone(), cx)
                })
            })
            .collect::<Vec<_>>();
        let title = if snapshot.keyword.is_empty() {
            "搜索".to_string()
        } else {
            format!("搜索: {}", snapshot.keyword)
        };
        let status = common::status_banner(
            snapshot.loading,
            snapshot.error.as_deref(),
            "搜索中...",
            "搜索失败",
        );
        let results = if rows.is_empty() {
            common::empty_card("暂无结果")
        } else {
            common::stacked_rows(rows, px(8.))
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(28.))
            .gap_5()
            .child(
                div()
                    .text_size(px(42.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                    .child(title),
            )
            .child(status)
            .child(results)
    }
}
