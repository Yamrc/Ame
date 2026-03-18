use nekowg::{
    AnyElement, App, Context, Entity, FontWeight, MouseButton, Render, Subscription, Window, div,
    img, prelude::*, px, relative, rgb,
};
use std::rc::Rc;
use std::sync::Arc;

use crate::action::library_actions::{LibraryPlaylistItem, PlaylistTrackItem};
use crate::component::cover_card::{self, CoverCardActions, PlaylistCoverCardProps};
use crate::component::short_track_item::{self, ShortTrackItemActions, ShortTrackItemProps};
use crate::component::{button, icon, theme};
use crate::entity::pages::DataState;
use crate::entity::player_controller::{PlayerController, QueueTrackInput};
use crate::entity::runtime::AppRuntime;
use crate::entity::services::pages;
use crate::router::{self, RouterState};
use crate::util::url::image_resize_url;
use crate::view::common;
use nekowg::SharedString;

const PREVIEW_COLS: usize = 3;
const PREVIEW_MAX: usize = 12;
const PREVIEW_ROW_HEIGHT: f32 = 52.0;
const PREVIEW_ROW_GAP: f32 = 8.0;
const PLAYLIST_GRID_COLUMNS: usize = 5;
type PreviewPlayHandler = Arc<dyn Fn(PlaylistTrackItem, &mut App)>;
type PlaylistActionHandler = Rc<dyn Fn(i64, &mut App)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPlaylistCard {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Created,
    Collected,
    Followed,
}

#[derive(Clone)]
pub struct LibraryPageSnapshot {
    pub title: SharedString,
    pub user_avatar: Option<String>,
    pub loading: bool,
    pub error: Option<SharedString>,
    pub liked_playlist: Option<LibraryPlaylistCard>,
    pub liked_lyric_lines: Vec<String>,
    pub liked_tracks: Vec<PlaylistTrackItem>,
    pub active_tab: LibraryTab,
    pub created_playlists: Vec<LibraryPlaylistCard>,
    pub collected_playlists: Vec<LibraryPlaylistCard>,
    pub followed_playlists: Vec<LibraryPlaylistCard>,
}

impl LibraryPageSnapshot {
    pub fn from_state(
        playlists_state: &DataState<Vec<LibraryPlaylistItem>>,
        liked_tracks_state: &DataState<Vec<PlaylistTrackItem>>,
        liked_lyric_lines: &[String],
        active_tab: LibraryTab,
        auth_account_summary: Option<&str>,
        auth_user_name: Option<&str>,
        auth_user_avatar: Option<&str>,
    ) -> Self {
        let liked_playlist = playlists_state
            .data
            .iter()
            .find(|item| item.special_type == 5)
            .map(Self::map_playlist_card);
        let created_playlists = playlists_state
            .data
            .iter()
            .filter(|item| !item.subscribed && item.special_type != 5)
            .map(Self::map_playlist_card)
            .collect();
        let collected_playlists = playlists_state
            .data
            .iter()
            .filter(|item| item.subscribed)
            .map(Self::map_playlist_card)
            .collect();
        let title = auth_user_name
            .filter(|name| !name.trim().is_empty())
            .map(|name| format!("{name} 的音乐库"))
            .or_else(|| {
                auth_account_summary
                    .filter(|summary| !summary.trim().is_empty())
                    .map(|summary| format!("{summary} 的音乐库"))
            })
            .unwrap_or_else(|| "我的音乐库".to_string());

        Self {
            title: title.into(),
            user_avatar: auth_user_avatar.map(ToOwned::to_owned),
            loading: playlists_state.loading,
            error: playlists_state.error.clone().map(Into::into),
            liked_playlist,
            liked_lyric_lines: liked_lyric_lines.to_vec(),
            liked_tracks: liked_tracks_state.data.clone(),
            active_tab,
            created_playlists,
            collected_playlists,
            followed_playlists: Vec::new(),
        }
    }

    fn map_playlist_card(item: &LibraryPlaylistItem) -> LibraryPlaylistCard {
        LibraryPlaylistCard {
            id: item.id,
            name: item.name.clone(),
            track_count: item.track_count,
            creator_name: item.creator_name.clone(),
            cover_url: item.cover_url.clone(),
        }
    }
}

pub fn liked_card(
    item: LibraryPlaylistCard,
    lyric_lines: &[String],
    min_height: nekowg::Pixels,
    on_open: impl Fn(&mut App) + 'static,
    on_play: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let top_lines = if lyric_lines.is_empty() {
        vec!["暂无喜欢歌曲".to_string()]
    } else {
        lyric_lines.iter().take(2).cloned().collect()
    };

    let play_button = div()
        .size(px(44.))
        .rounded_full()
        .bg(rgb(theme::COLOR_PRIMARY))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .child(icon::render(
            icon::IconName::Play,
            16.,
            theme::COLOR_PRIMARY_BG_DARK,
        ))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            cx.stop_propagation();
            on_play(cx);
        });

    div()
        .w(relative(0.330))
        .cursor_pointer()
        .rounded_2xl()
        .px(px(24.))
        .py(px(18.))
        .bg(rgb(theme::COLOR_PRIMARY_BG_DARK))
        .min_h(min_height)
        .flex()
        .flex_col()
        .justify_between()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.))
                .text_size(px(14.))
                .line_height(relative(1.2))
                .font_weight(FontWeight::LIGHT)
                .text_color(rgb(theme::COLOR_PRIMARY))
                .children(
                    top_lines
                        .into_iter()
                        .map(|line| div().child(line).into_any_element()),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_size(px(24.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(theme::COLOR_PRIMARY))
                                .line_height(relative(1.2))
                                .child(item.name),
                        )
                        .child(
                            div()
                                .text_size(px(15.))
                                .mt(px(2.))
                                .line_height(relative(1.2))
                                .text_color(rgb(theme::COLOR_PRIMARY))
                                .child(format!("{} 首歌", item.track_count)),
                        ),
                )
                .child(play_button),
        )
        .into_any_element()
}

fn empty_liked_card(min_height: nekowg::Pixels) -> AnyElement {
    div()
        .w(relative(0.330))
        .min_h(min_height)
        .child(common::empty_card("暂无喜欢的音乐"))
        .into_any_element()
}

fn liked_preview_list(
    tracks: &[PlaylistTrackItem],
    row_height: f32,
    row_gap: f32,
    on_play: PreviewPlayHandler,
) -> AnyElement {
    if tracks.is_empty() {
        return common::empty_card("暂无喜欢歌曲");
    }

    div()
        .overflow_hidden()
        .grid()
        .grid_cols(PREVIEW_COLS as u16)
        .gap(px(row_gap))
        .children(tracks.iter().take(PREVIEW_MAX).map(|track| {
            let track_for_play = track.clone();
            let on_play = on_play.clone();
            short_track_item::render(
                ShortTrackItemProps {
                    id: track.id,
                    title: track.name.clone(),
                    subtitle: track.artists.clone(),
                    cover_url: track.cover_url.clone(),
                    height: px(row_height),
                },
                ShortTrackItemActions {
                    on_play: Some(Rc::new(move |cx| on_play(track_for_play.clone(), cx))),
                    ..ShortTrackItemActions::default()
                },
            )
        }))
        .into_any_element()
}

fn build_header(title: &SharedString, user_avatar: Option<String>) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .child(match user_avatar {
            Some(url) => img(image_resize_url(&url, "96y96"))
                .size(px(44.))
                .rounded_full()
                .overflow_hidden()
                .into_any_element(),
            None => div()
                .size(px(44.))
                .rounded_full()
                .bg(rgb(theme::COLOR_CARD_DARK))
                .into_any_element(),
        })
        .child(
            div()
                .text_size(px(42.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_TEXT_DARK))
                .child(title.clone()),
        )
        .into_any_element()
}

fn render_tabs(
    active_tab: LibraryTab,
    on_tab_created: Arc<dyn Fn(&mut App)>,
    on_tab_collected: Arc<dyn Fn(&mut App)>,
    on_tab_followed: Arc<dyn Fn(&mut App)>,
) -> AnyElement {
    let on_tab_created = on_tab_created.clone();
    let on_tab_collected = on_tab_collected.clone();
    let on_tab_followed = on_tab_followed.clone();
    div()
        .flex()
        .gap(px(12.))
        .child(
            button::chip_base("创建的歌单", active_tab == LibraryTab::Created)
                .on_mouse_down(MouseButton::Left, move |_, _, cx| on_tab_created(cx)),
        )
        .child(
            button::chip_base("收藏的歌单", active_tab == LibraryTab::Collected)
                .on_mouse_down(MouseButton::Left, move |_, _, cx| on_tab_collected(cx)),
        )
        .child(
            button::chip_base("关注内容", active_tab == LibraryTab::Followed)
                .on_mouse_down(MouseButton::Left, move |_, _, cx| on_tab_followed(cx)),
        )
        .into_any_element()
}

fn render_tab_panel(cards: Vec<AnyElement>, empty_label: &str) -> AnyElement {
    if cards.is_empty() {
        return common::empty_card(empty_label.to_string());
    }
    cards
        .into_iter()
        .fold(
            div()
                .w_full()
                .grid()
                .grid_cols(PLAYLIST_GRID_COLUMNS as u16)
                .gap(px(18.)),
            |grid, card| grid.child(card),
        )
        .into_any_element()
}

fn build_playlist_cards(
    playlists: &[LibraryPlaylistCard],
    on_open_playlist: PlaylistActionHandler,
) -> Vec<AnyElement> {
    playlists
        .iter()
        .cloned()
        .map(|item| {
            let playlist_id = item.id;
            let on_open_playlist = on_open_playlist.clone();
            cover_card::render_playlist_card(
                PlaylistCoverCardProps {
                    title: item.name,
                    subtitle: format!("{} 首 · {}", item.track_count, item.creator_name),
                    cover_url: item.cover_url,
                    cover_height: px(166.),
                },
                CoverCardActions {
                    on_open: Some(Rc::new(move |cx| on_open_playlist(playlist_id, cx))),
                },
            )
        })
        .collect()
}

pub struct LibraryPageView {
    runtime: AppRuntime,
    player_controller: Entity<PlayerController>,
    observed_user_id: Option<i64>,
    _subscriptions: Vec<Subscription>,
}

impl LibraryPageView {
    pub fn new(
        runtime: AppRuntime,
        player_controller: Entity<PlayerController>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe_global::<RouterState>(|this, cx| {
            if this.is_active(cx) {
                this.ensure_loaded(cx);
            }
        }));
        subscriptions.push(cx.observe(&runtime.session, |this, _, cx| {
            this.handle_session_change(cx);
        }));
        let observed_user_id = runtime.session.read(cx).auth_user_id;
        let mut this = Self {
            runtime,
            player_controller,
            observed_user_id,
            _subscriptions: subscriptions,
        };
        if this.is_active(cx) {
            this.ensure_loaded(cx);
        }
        this
    }

    fn is_active(&self, cx: &mut Context<Self>) -> bool {
        router::current_path(cx).as_ref() == "/library"
    }

    fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(false, cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(true, cx);
    }

    fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let user_id = self.runtime.session.read(cx).auth_user_id;
        let changed = self.observed_user_id != user_id;
        self.observed_user_id = user_id;

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
        let Some(user_id) = session.auth_user_id else {
            self.runtime.library.update(cx, |library, cx| {
                library.playlists.clear();
                library.liked_tracks.clear();
                library.liked_lyric_lines.clear();
                cx.notify();
            });
            return;
        };

        let mut library = self.runtime.library.read(cx).clone();
        if !force {
            if library.playlists.loading {
                return;
            }
            if library.playlists.fetched_at_ms.is_some() {
                return;
            }
        }

        let Some(cookie) = session
            .auth_bundle
            .music_u
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .and_then(|_| crate::action::auth_actions::build_cookie_header(&session.auth_bundle))
        else {
            library.playlists.fail("缺少鉴权凭据");
            library.liked_tracks.clear();
            library.liked_lyric_lines.clear();
            self.runtime.library.update(cx, |state, cx| {
                *state = library;
                cx.notify();
            });
            return;
        };

        library
            .playlists
            .begin(crate::entity::pages::DataSource::User);
        library
            .liked_tracks
            .begin(crate::entity::pages::DataSource::User);
        library.liked_lyric_lines.clear();
        self.runtime.library.update(cx, |state, cx| {
            *state = library;
            cx.notify();
        });

        let page = cx.entity();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { pages::fetch_library_payload(user_id, &cookie) })
                .await;

            page.update(cx, |this, cx| {
                this.apply_loaded_library(user_id, result, cx);
            });
        })
        .detach();
    }

    fn apply_loaded_library(
        &mut self,
        user_id: i64,
        result: Result<pages::LibraryLoadResult, String>,
        cx: &mut Context<Self>,
    ) {
        if self.runtime.session.read(cx).auth_user_id != Some(user_id) {
            return;
        }

        self.runtime.library.update(cx, |library, cx| {
            match result {
                Ok(result) => {
                    library
                        .playlists
                        .succeed(result.playlists, Some(result.fetched_at_ms));
                    library
                        .liked_tracks
                        .succeed(result.liked_tracks, Some(result.fetched_at_ms));
                    library.liked_lyric_lines = result.liked_lyric_lines;
                }
                Err(err) => {
                    library.playlists.clear();
                    library.playlists.fail(err.clone());
                    library.liked_tracks.clear();
                    library.liked_tracks.fail(err);
                    library.liked_lyric_lines.clear();
                }
            }
            cx.notify();
        });
    }

    fn set_tab(&mut self, tab: LibraryTab, cx: &mut Context<Self>) {
        self.runtime.library.update(cx, |state, cx| {
            state.tab = tab;
            cx.notify();
        });
    }

    fn replace_queue_from_playlist(&mut self, playlist_id: i64, cx: &mut Context<Self>) {
        let page = match pages::ensure_playlist_page_loaded(&self.runtime, playlist_id, cx) {
            Ok(page) => page,
            Err(err) => {
                self.runtime.shell.update(cx, |shell, cx| {
                    shell.error = Some(format!("替换队列失败: {err}"));
                    cx.notify();
                });
                return;
            }
        };
        let tracks = page
            .tracks
            .into_iter()
            .map(QueueTrackInput::from)
            .collect::<Vec<_>>();
        self.player_controller
            .update(cx, |player, cx| player.replace_queue(tracks, 0, cx));
    }
}

impl Render for LibraryPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let library = self.runtime.library.read(cx).clone();
        let session = self.runtime.session.read(cx).clone();
        let snapshot = LibraryPageSnapshot::from_state(
            &library.playlists,
            &library.liked_tracks,
            &library.liked_lyric_lines,
            library.tab,
            session.auth_account_summary.as_deref(),
            session.auth_user_name.as_deref(),
            session.auth_user_avatar.as_deref(),
        );
        let page = cx.entity();
        let on_open_playlist: PlaylistActionHandler = {
            let page = page.clone();
            Rc::new(move |playlist_id, cx| {
                page.update(cx, |_, cx| {
                    router::navigate(cx, format!("/playlist/{playlist_id}"));
                });
            })
        };
        let on_replace_queue_from_playlist: PlaylistActionHandler = {
            let page = page.clone();
            Rc::new(move |playlist_id, cx| {
                page.update(cx, |this, cx| {
                    this.replace_queue_from_playlist(playlist_id, cx)
                });
            })
        };
        let on_preview_play: PreviewPlayHandler = {
            let player_controller = self.player_controller.clone();
            Arc::new(move |track, cx| {
                player_controller.update(cx, |this, cx| {
                    this.enqueue_track(track.into(), true, cx);
                });
            })
        };
        let on_tab_created = {
            let page = page.clone();
            Arc::new(move |cx: &mut App| {
                page.update(cx, |this, cx| this.set_tab(LibraryTab::Created, cx));
            })
        };
        let on_tab_collected = {
            let page = page.clone();
            Arc::new(move |cx: &mut App| {
                page.update(cx, |this, cx| this.set_tab(LibraryTab::Collected, cx));
            })
        };
        let on_tab_followed = {
            let page = page.clone();
            Arc::new(move |cx: &mut App| {
                page.update(cx, |this, cx| this.set_tab(LibraryTab::Followed, cx));
            })
        };

        let preview_count = snapshot.liked_tracks.len().min(PREVIEW_MAX);
        let preview_rows = preview_count.div_ceil(PREVIEW_COLS).max(2);
        let preview_height = preview_rows as f32 * PREVIEW_ROW_HEIGHT
            + (preview_rows.saturating_sub(1) as f32) * PREVIEW_ROW_GAP;
        let preview_min_height = px(preview_height);

        let liked_card = snapshot.liked_playlist.clone().map(|item| {
            let playlist_id = item.id;
            let on_open_playlist = on_open_playlist.clone();
            let on_replace_queue_from_playlist = on_replace_queue_from_playlist.clone();
            liked_card(
                item,
                &snapshot.liked_lyric_lines,
                preview_min_height,
                move |cx| on_open_playlist(playlist_id, cx),
                move |cx| on_replace_queue_from_playlist(playlist_id, cx),
            )
        });
        let created_cards =
            build_playlist_cards(&snapshot.created_playlists, on_open_playlist.clone());
        let collected_cards =
            build_playlist_cards(&snapshot.collected_playlists, on_open_playlist.clone());
        let followed_cards =
            build_playlist_cards(&snapshot.followed_playlists, on_open_playlist.clone());

        let status = common::status_banner(
            snapshot.loading,
            snapshot.error.as_ref().map(AsRef::as_ref),
            "加载中...",
            "加载失败",
        );
        let liked_card = liked_card.unwrap_or_else(|| empty_liked_card(preview_min_height));
        let liked_preview = liked_preview_list(
            &snapshot.liked_tracks,
            PREVIEW_ROW_HEIGHT,
            PREVIEW_ROW_GAP,
            on_preview_play,
        );
        let header = build_header(&snapshot.title, snapshot.user_avatar);

        let tabs = render_tabs(
            snapshot.active_tab,
            on_tab_created,
            on_tab_collected,
            on_tab_followed,
        );
        let panel = match snapshot.active_tab {
            LibraryTab::Created => render_tab_panel(created_cards, "暂无创建歌单"),
            LibraryTab::Collected => render_tab_panel(collected_cards, "暂无收藏歌单"),
            LibraryTab::Followed => render_tab_panel(followed_cards, "暂无关注"),
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(20.))
            .child(header)
            .child(
                div()
                    .w_full()
                    .mt(px(20.))
                    .flex()
                    .items_center()
                    .child(liked_card)
                    .child(div().w(relative(0.671)).ml(px(36.)).child(liked_preview)),
            )
            .child(div().w_full().mt(px(20.)).child(status))
            .child(div().w_full().mt(px(20.)).child(tabs))
            .child(div().w_full().mt(px(16.)).child(panel))
            .into_any_element()
    }
}
