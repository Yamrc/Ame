use std::sync::Arc;

use nekowg::{
    AnyElement, App, Context, FontWeight, Render, Subscription, Window, div, prelude::*, px, rgb,
};

use crate::action::library_actions::LibraryPlaylistItem;
use crate::component::button;
use crate::component::playlist_item::{self, PlaylistItemActions, PlaylistItemProps};
use crate::component::theme;
use crate::entity::pages::DataState;
use crate::entity::runtime::AppRuntime;
use crate::entity::services::pages;
use crate::router::{self, RouterState};
use crate::view::common;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverPlaylistCard {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiscoverPageSnapshot {
    pub loading: bool,
    pub error: Option<String>,
    pub playlists: Vec<DiscoverPlaylistCard>,
}

impl DiscoverPageSnapshot {
    pub fn from_state(state: &DataState<Vec<LibraryPlaylistItem>>) -> Self {
        Self {
            loading: state.loading,
            error: state.error.clone(),
            playlists: state
                .data
                .iter()
                .take(12)
                .map(|item| DiscoverPlaylistCard {
                    id: item.id,
                    name: item.name.clone(),
                    track_count: item.track_count,
                    creator_name: item.creator_name.clone(),
                    cover_url: item.cover_url.clone(),
                })
                .collect(),
        }
    }
}

pub fn playlist_card(
    item: DiscoverPlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
) -> AnyElement {
    playlist_item::render(
        PlaylistItemProps {
            id: item.id,
            name: item.name,
            creator: item.creator_name,
            track_count: Some(item.track_count),
            cover_url: item.cover_url,
            cover_size: px(58.),
        },
        PlaylistItemActions {
            on_open: Arc::new(on_open),
        },
    )
}

fn chip(text: &'static str, active: bool) -> impl IntoElement {
    button::chip_base(text, active)
        .mr(px(12.))
        .mt(px(8.))
        .mb(px(4.))
        .hover(|this| {
            this.bg(rgb(theme::COLOR_PRIMARY_BG_DARK))
                .text_color(rgb(theme::COLOR_PRIMARY))
        })
}

pub struct DiscoverPageView {
    runtime: AppRuntime,
    last_user_token_state: bool,
    _subscriptions: Vec<Subscription>,
}

impl DiscoverPageView {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe_global::<RouterState>(|this, cx| {
            if this.is_active(cx) {
                this.ensure_loaded(cx);
            }
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        let last_user_token_state = runtime
            .session
            .read(cx)
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let mut this = Self {
            runtime,
            last_user_token_state,
            _subscriptions: subscriptions,
        };
        if this.is_active(cx) {
            this.ensure_loaded(cx);
        }
        this
    }

    fn is_active(&self, cx: &mut Context<Self>) -> bool {
        router::current_path(cx).as_ref() == "/explore"
    }

    fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(false, cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(true, cx);
    }

    fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let has_user_token = self
            .runtime
            .session
            .read(cx)
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let changed = self.last_user_token_state != has_user_token;
        self.last_user_token_state = has_user_token;
        if !self.is_active(cx) {
            return;
        }
        if changed {
            self.reload(cx);
        } else {
            cx.notify();
        }
    }

    fn load(&mut self, force: bool, cx: &mut Context<Self>) {
        let session = self.runtime.session.read(cx).clone();
        let has_user_token = session
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty());
        let source = if has_user_token {
            crate::entity::pages::DataSource::User
        } else {
            crate::entity::pages::DataSource::Guest
        };
        let state = self.runtime.discover.read(cx).clone();
        if !force {
            if state.playlists.loading {
                return;
            }
            if state.playlists.source == source && state.playlists.fetched_at_ms.is_some() {
                return;
            }
        }

        let Some(cookie) = crate::action::auth_actions::build_cookie_header(&session.auth_bundle)
        else {
            self.runtime.discover.update(cx, |discover, cx| {
                discover.playlists.fail("缺少鉴权凭据");
                cx.notify();
            });
            return;
        };

        self.runtime.discover.update(cx, |discover, cx| {
            discover.playlists.begin(source);
            cx.notify();
        });

        let page = cx.entity();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { pages::fetch_discover_payload(&cookie) })
                .await;
            page.update(cx, |this, cx| this.apply_load_result(source, result, cx));
        })
        .detach();
    }

    fn apply_load_result(
        &mut self,
        source: crate::entity::pages::DataSource,
        result: Result<pages::DiscoverLoadResult, String>,
        cx: &mut Context<Self>,
    ) {
        self.runtime.discover.update(cx, |discover, cx| {
            match result {
                Ok(result) => discover
                    .playlists
                    .succeed(result.playlists, Some(result.fetched_at_ms)),
                Err(err) => {
                    discover.playlists.clear();
                    discover.playlists.fail(err);
                }
            }
            discover.playlists.source = source;
            cx.notify();
        });
    }
}

impl Render for DiscoverPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = DiscoverPageSnapshot::from_state(&self.runtime.discover.read(cx).playlists);
        let page = cx.entity();
        let rows = snapshot
            .playlists
            .into_iter()
            .map(|item| {
                let playlist_id = item.id;
                let page = page.clone();
                playlist_card(item, move |cx| {
                    page.update(cx, |_, cx| {
                        router::navigate(cx, format!("/playlist/{playlist_id}"));
                    });
                })
            })
            .collect::<Vec<_>>();
        let status = common::status_banner(
            snapshot.loading,
            snapshot.error.as_deref(),
            "加载中...",
            "加载失败",
        );
        let playlist_section = if rows.is_empty() {
            div()
                .w_full()
                .rounded_xl()
                .bg(rgb(theme::COLOR_CARD_DARK))
                .p_5()
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child("暂无推荐内容")
                .into_any_element()
        } else {
            common::stacked_rows(rows, px(8.))
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(28.))
            .child(
                div()
                    .text_size(px(56.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                    .child("发现"),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .mt(px(4.))
                    .mb(px(16.))
                    .child(chip("全部", true))
                    .child(chip("推荐歌单", false))
                    .child(chip("排行榜", false))
                    .child(chip("流行", false)),
            )
            .child(status)
            .child(playlist_section)
    }
}
