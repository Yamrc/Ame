use std::rc::Rc;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use nekowg::{
    AnyElement, App, Context, Entity, FontWeight, MouseButton, Render, Subscription, Window, div,
    prelude::*, px, rgb,
};

use crate::component::{
    button,
    cover_card::{self, ArtistCoverCardProps, CoverCardActions, PlaylistCoverCardProps},
    short_track_item::{self, ShortTrackItemActions, ShortTrackItemProps},
    theme,
    track_item::{self, TrackItemActions, TrackItemProps},
};
use crate::entity::pages::{DataSource, SearchCollectionState};
use crate::entity::player_controller::PlayerController;
use crate::entity::runtime::AppRuntime;
use crate::entity::services::pages;
use crate::router::{self, RouterState, use_params};
use crate::view::common;

const TYPE_PAGE_LIMIT: u32 = 30;
const PLAYLIST_CARD_HEIGHT: f32 = 166.0;
const PLAYLIST_GRID_COLUMNS: usize = 6;
const SHORT_TRACK_COLUMNS: usize = 4;
const SHORT_TRACK_HEIGHT: f32 = 48.0;
const SHORT_TRACK_GRID_GAP: f32 = 12.0;
const SEARCH_TYPE_CARD_COLUMNS: usize = 5;
const OVERVIEW_ARTIST_PLACEHOLDER_HEIGHT: f32 = 180.0;
const OVERVIEW_CARD_PLACEHOLDER_HEIGHT: f32 = 166.0;
const OVERVIEW_TRACK_PLACEHOLDER_HEIGHT: f32 = SHORT_TRACK_HEIGHT + 8.0;

type PlaySongHandler = Arc<dyn Fn(SearchSong, &mut App)>;
type EnqueueSongHandler = Arc<dyn Fn(SearchSong, &mut App)>;
type NavigateHandler = Rc<dyn Fn(&mut App)>;
type PlaylistOpenHandler = Rc<dyn Fn(i64, &mut App)>;
type SearchTypeNavigateHandler = Rc<dyn Fn(SearchRouteType, &mut App)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchRouteType {
    Artists,
    Albums,
    Tracks,
    Playlists,
}

impl SearchRouteType {
    pub fn from_param(value: &str) -> Option<Self> {
        match value.trim() {
            "artists" => Some(Self::Artists),
            "albums" => Some(Self::Albums),
            "tracks" => Some(Self::Tracks),
            "playlists" => Some(Self::Playlists),
            _ => None,
        }
    }

    pub const fn path_segment(self) -> &'static str {
        match self {
            Self::Artists => "artists",
            Self::Albums => "albums",
            Self::Tracks => "tracks",
            Self::Playlists => "playlists",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Artists => "艺人",
            Self::Albums => "专辑",
            Self::Tracks => "歌曲",
            Self::Playlists => "歌单",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SearchRouteKey {
    keyword: String,
    route_type: Option<SearchRouteType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSong {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchArtist {
    pub id: i64,
    pub name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAlbum {
    pub id: i64,
    pub name: String,
    pub artist_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPlaylist {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchOverview {
    pub artists: Vec<SearchArtist>,
    pub albums: Vec<SearchAlbum>,
    pub tracks: Vec<SearchSong>,
    pub playlists: Vec<SearchPlaylist>,
}

impl SearchOverview {
    fn has_result(&self) -> bool {
        !self.artists.is_empty()
            || !self.albums.is_empty()
            || !self.tracks.is_empty()
            || !self.playlists.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct SearchPageSlice<T> {
    pub items: Vec<T>,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
pub enum SearchTypePayload {
    Artists(SearchPageSlice<SearchArtist>),
    Albums(SearchPageSlice<SearchAlbum>),
    Tracks(SearchPageSlice<SearchSong>),
    Playlists(SearchPageSlice<SearchPlaylist>),
}

fn render_track_row(
    song: SearchSong,
    is_playing: bool,
    on_play: impl Fn(&mut App) + 'static,
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
            cover_url: song.cover_url,
            show_cover: true,
            is_playing,
        },
        TrackItemActions {
            on_play: Some(Rc::new(on_play)),
            on_enqueue: Some(Rc::new(on_enqueue)),
            ..TrackItemActions::default()
        },
    )
}

fn render_short_track_item(
    song: SearchSong,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
) -> AnyElement {
    short_track_item::render(
        ShortTrackItemProps {
            id: song.id,
            title: song.name,
            subtitle: song.artists,
            cover_url: song.cover_url,
            height: px(SHORT_TRACK_HEIGHT),
        },
        ShortTrackItemActions {
            on_play: Some(Rc::new(on_play)),
            on_enqueue: Some(Rc::new(on_enqueue)),
        },
    )
}

fn section_header(title: &str, on_more: Option<NavigateHandler>) -> AnyElement {
    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_size(px(26.))
                .font_weight(FontWeight::BOLD)
                .child(title.to_string()),
        )
        .child(match on_more {
            Some(on_more) => div()
                .cursor_pointer()
                .text_size(px(14.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child("查看全部")
                .on_mouse_down(MouseButton::Left, move |_, _, cx| on_more(cx))
                .into_any_element(),
            None => div().into_any_element(),
        })
        .into_any_element()
}

fn render_grid(items: Vec<AnyElement>, columns: usize, empty_label: &str) -> AnyElement {
    if items.is_empty() {
        return common::empty_card(empty_label.to_string());
    }

    items
        .into_iter()
        .fold(
            div().w_full().grid().grid_cols(columns as u16).gap(px(24.)),
            |grid, item| grid.child(item),
        )
        .into_any_element()
}

fn render_grid_with_gap(
    items: Vec<AnyElement>,
    columns: usize,
    gap: f32,
    empty_label: &str,
) -> AnyElement {
    if items.is_empty() {
        return common::empty_card(empty_label.to_string());
    }

    items
        .into_iter()
        .fold(
            div().w_full().grid().grid_cols(columns as u16).gap(px(gap)),
            |grid, item| grid.child(item),
        )
        .into_any_element()
}

fn render_overview_placeholder(label: &str, min_height: f32) -> AnyElement {
    div()
        .w_full()
        .min_h(px(min_height))
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(theme::COLOR_SECONDARY))
        .child(label.to_string())
        .into_any_element()
}

fn render_overview_grid(
    items: Vec<AnyElement>,
    columns: usize,
    gap: Option<f32>,
    empty_label: &str,
    empty_min_height: f32,
) -> AnyElement {
    if items.is_empty() {
        render_overview_placeholder(empty_label, empty_min_height)
    } else {
        match gap {
            Some(gap) => render_grid_with_gap(items, columns, gap, empty_label),
            None => render_grid(items, columns, empty_label),
        }
    }
}

fn should_skip_collection_load<T>(
    state: &SearchCollectionState<T>,
    keyword: &str,
    append: bool,
) -> bool {
    if append {
        state.items.loading || !state.has_more || state.keyword != keyword
    } else {
        state.items.loading || (state.keyword == keyword && state.items.fetched_at_ms.is_some())
    }
}

fn prepare_collection_load<T>(
    state: &mut SearchCollectionState<T>,
    keyword: String,
    append: bool,
    source: DataSource,
) {
    if !append {
        state.keyword = keyword;
        state.items.data.clear();
        state.items.fetched_at_ms = None;
        state.has_more = true;
    }
    state.items.begin(source);
}

fn apply_collection_result<T>(
    state: &mut SearchCollectionState<T>,
    keyword: String,
    page: SearchPageSlice<T>,
    append: bool,
) {
    state.keyword = keyword;
    state.has_more = page.has_more;
    if append {
        state.items.data.extend(page.items);
        state.items.loading = false;
        state.items.error = None;
        state.items.fetched_at_ms = Some(now_millis());
    } else {
        state.items.succeed(page.items, Some(now_millis()));
    }
}

fn apply_collection_error<T>(
    state: &mut SearchCollectionState<T>,
    keyword: String,
    error: String,
    append: bool,
) {
    state.keyword = keyword;
    if !append {
        state.items.clear();
    }
    state.items.fail(error);
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub struct SearchPageView {
    runtime: AppRuntime,
    player_controller: Entity<PlayerController>,
    last_route: SearchRouteKey,
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
            this.handle_session_change(cx);
        }));
        subscriptions.push(cx.observe(&runtime.player, |_, _, cx| {
            cx.notify();
        }));
        let mut this = Self {
            runtime,
            player_controller,
            last_route: SearchRouteKey::default(),
            _subscriptions: subscriptions,
        };
        this.sync_route_query(cx);
        this
    }

    fn is_active(&self, cx: &mut Context<Self>) -> bool {
        router::current_path(cx).as_ref().starts_with("/search")
    }

    fn current_route(&self, cx: &mut Context<Self>) -> SearchRouteKey {
        if !self.is_active(cx) {
            return SearchRouteKey::default();
        }
        let params = use_params(cx);
        let keyword = params
            .get("keywords")
            .map(|value| value.as_ref().to_string())
            .unwrap_or_default();
        let route_type = params
            .get("type")
            .and_then(|value| SearchRouteType::from_param(value.as_ref()));
        SearchRouteKey {
            keyword,
            route_type,
        }
    }

    fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        if !self.is_active(cx) {
            return;
        }
        self.last_route = SearchRouteKey::default();
        self.sync_route_query(cx);
    }

    fn sync_route_query(&mut self, cx: &mut Context<Self>) {
        if !self.is_active(cx) {
            return;
        }

        let route = self.current_route(cx);
        if route.keyword.trim().is_empty() {
            self.last_route = route;
            self.clear_search_state(cx);
            return;
        }
        if route == self.last_route {
            return;
        }

        self.last_route = route.clone();
        match route.route_type {
            None => self.ensure_overview_loaded(route.keyword, cx),
            Some(route_type) => self.ensure_type_loaded(route.keyword, route_type, false, cx),
        }
    }

    fn clear_search_state(&mut self, cx: &mut Context<Self>) {
        self.runtime.search.update(cx, |search, cx| {
            search.overview_keyword.clear();
            search.overview.clear();
            search.artists.clear();
            search.albums.clear();
            search.tracks.clear();
            search.playlists.clear();
            cx.notify();
        });
    }

    fn data_source(&self, cx: &mut Context<Self>) -> DataSource {
        if self
            .runtime
            .session
            .read(cx)
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            DataSource::User
        } else {
            DataSource::Guest
        }
    }

    fn auth_cookie(&self, cx: &mut Context<Self>) -> Option<String> {
        let session = self.runtime.session.read(cx).clone();
        crate::action::auth_actions::build_cookie_header(&session.auth_bundle)
    }

    fn ensure_overview_loaded(&mut self, keyword: String, cx: &mut Context<Self>) {
        let state = self.runtime.search.read(cx).clone();
        if state.overview.loading {
            return;
        }
        if state.overview_keyword == keyword && state.overview.fetched_at_ms.is_some() {
            return;
        }

        let source = self.data_source(cx);
        let cookie = self.auth_cookie(cx);
        self.runtime.search.update(cx, |search, cx| {
            search.overview_keyword = keyword.clone();
            search.overview.begin(source);
            cx.notify();
        });

        let page = cx.entity();
        let request_keyword = keyword.clone();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    pages::fetch_search_overview_payload(&request_keyword, cookie.as_deref())
                })
                .await;
            page.update(cx, |this, cx| {
                this.apply_overview_result(keyword, result, cx)
            });
        })
        .detach();
    }

    fn ensure_type_loaded(
        &mut self,
        keyword: String,
        route_type: SearchRouteType,
        append: bool,
        cx: &mut Context<Self>,
    ) {
        let state = self.runtime.search.read(cx).clone();
        let current_len = match route_type {
            SearchRouteType::Artists => state.artists.items.data.len(),
            SearchRouteType::Albums => state.albums.items.data.len(),
            SearchRouteType::Tracks => state.tracks.items.data.len(),
            SearchRouteType::Playlists => state.playlists.items.data.len(),
        };
        let should_skip = match route_type {
            SearchRouteType::Artists => {
                should_skip_collection_load(&state.artists, &keyword, append)
            }
            SearchRouteType::Albums => should_skip_collection_load(&state.albums, &keyword, append),
            SearchRouteType::Tracks => should_skip_collection_load(&state.tracks, &keyword, append),
            SearchRouteType::Playlists => {
                should_skip_collection_load(&state.playlists, &keyword, append)
            }
        };
        if should_skip {
            return;
        }

        let source = self.data_source(cx);
        let cookie = self.auth_cookie(cx);
        self.runtime.search.update(cx, |search, cx| {
            match route_type {
                SearchRouteType::Artists => {
                    prepare_collection_load(&mut search.artists, keyword.clone(), append, source)
                }
                SearchRouteType::Albums => {
                    prepare_collection_load(&mut search.albums, keyword.clone(), append, source)
                }
                SearchRouteType::Tracks => {
                    prepare_collection_load(&mut search.tracks, keyword.clone(), append, source)
                }
                SearchRouteType::Playlists => {
                    prepare_collection_load(&mut search.playlists, keyword.clone(), append, source)
                }
            }
            cx.notify();
        });

        let offset = if append { current_len as u32 } else { 0 };
        let page = cx.entity();
        let request_keyword = keyword.clone();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    pages::fetch_search_type_payload(
                        &request_keyword,
                        route_type,
                        offset,
                        TYPE_PAGE_LIMIT,
                        cookie.as_deref(),
                    )
                })
                .await;
            page.update(cx, |this, cx| {
                this.apply_type_result(keyword, route_type, append, result, cx)
            });
        })
        .detach();
    }

    fn apply_overview_result(
        &mut self,
        keyword: String,
        result: Result<SearchOverview, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.is_active(cx) || self.current_route(cx).keyword != keyword {
            return;
        }

        self.runtime.search.update(cx, |search, cx| {
            match result {
                Ok(overview) => {
                    search.overview_keyword = keyword;
                    search.overview.succeed(overview, Some(now_millis()));
                }
                Err(err) => {
                    search.overview_keyword = keyword;
                    search.overview.clear();
                    search.overview.fail(err);
                }
            }
            cx.notify();
        });
    }

    fn apply_type_result(
        &mut self,
        keyword: String,
        route_type: SearchRouteType,
        append: bool,
        result: Result<SearchTypePayload, String>,
        cx: &mut Context<Self>,
    ) {
        let current_route = self.current_route(cx);
        if !self.is_active(cx)
            || current_route.keyword != keyword
            || current_route.route_type != Some(route_type)
        {
            return;
        }

        self.runtime.search.update(cx, |search, cx| {
            match (route_type, result) {
                (SearchRouteType::Artists, Ok(SearchTypePayload::Artists(page))) => {
                    apply_collection_result(&mut search.artists, keyword, page, append)
                }
                (SearchRouteType::Albums, Ok(SearchTypePayload::Albums(page))) => {
                    apply_collection_result(&mut search.albums, keyword, page, append)
                }
                (SearchRouteType::Tracks, Ok(SearchTypePayload::Tracks(page))) => {
                    apply_collection_result(&mut search.tracks, keyword, page, append)
                }
                (SearchRouteType::Playlists, Ok(SearchTypePayload::Playlists(page))) => {
                    apply_collection_result(&mut search.playlists, keyword, page, append)
                }
                (_, Ok(_)) => {}
                (SearchRouteType::Artists, Err(err)) => {
                    apply_collection_error(&mut search.artists, keyword, err, append)
                }
                (SearchRouteType::Albums, Err(err)) => {
                    apply_collection_error(&mut search.albums, keyword, err, append)
                }
                (SearchRouteType::Tracks, Err(err)) => {
                    apply_collection_error(&mut search.tracks, keyword, err, append)
                }
                (SearchRouteType::Playlists, Err(err)) => {
                    apply_collection_error(&mut search.playlists, keyword, err, append)
                }
            }
            cx.notify();
        });
    }

    fn load_more(&mut self, cx: &mut Context<Self>) {
        let route = self.current_route(cx);
        if let Some(route_type) = route.route_type {
            self.ensure_type_loaded(route.keyword, route_type, true, cx);
        }
    }
}

impl Render for SearchPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let route = self.current_route(cx);
        let page = cx.entity();
        let search_state = self.runtime.search.read(cx).clone();
        let current_playing_track_id = self
            .runtime
            .player
            .read(cx)
            .current_item()
            .map(|item| item.id);
        let on_play_song: PlaySongHandler = {
            let player_controller = self.player_controller.clone();
            Arc::new(move |song, cx| {
                player_controller.update(cx, |this, cx| this.enqueue_track(song.into(), true, cx));
            })
        };
        let on_enqueue_song: EnqueueSongHandler = {
            let player_controller = self.player_controller.clone();
            Arc::new(move |song, cx| {
                player_controller.update(cx, |this, cx| this.enqueue_track(song.into(), false, cx));
            })
        };
        let on_open_playlist: PlaylistOpenHandler = Rc::new(move |playlist_id, cx| {
            page.update(cx, |_, cx| {
                router::navigate(cx, format!("/playlist/{playlist_id}"));
            });
        });
        let page = cx.entity();
        let on_navigate_type: SearchTypeNavigateHandler = Rc::new(move |route_type, cx| {
            let params = cx.read_global(|state: &RouterState, _| state.params.clone());
            let keyword = params
                .get("keywords")
                .map(|value| value.as_ref().to_string())
                .unwrap_or_default();
            page.update(cx, |_, cx| {
                router::navigate(
                    cx,
                    format!("/search/{keyword}/{}", route_type.path_segment()),
                );
            });
        });

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
                let status = common::status_banner(
                    search_state.overview.loading,
                    search_state.overview.error.as_deref(),
                    "搜索中...",
                    "搜索失败",
                );
                let body = if route.keyword.is_empty() {
                    common::empty_card("输入关键字搜索")
                } else if !search_state.overview.data.has_result()
                    && !search_state.overview.loading
                    && search_state.overview.error.is_none()
                {
                    common::empty_card("暂无结果")
                } else {
                    render_overview_sections(
                        search_state.overview.data,
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
                &search_state,
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

fn render_overview_sections(
    overview: SearchOverview,
    on_play_song: PlaySongHandler,
    on_enqueue_song: EnqueueSongHandler,
    on_open_playlist: PlaylistOpenHandler,
    on_navigate_type: SearchTypeNavigateHandler,
) -> AnyElement {
    let on_navigate_type_for_artists = on_navigate_type.clone();
    let artists = div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .gap_4()
        .child(section_header(
            "艺人",
            Some(Rc::new(move |cx| {
                on_navigate_type_for_artists(SearchRouteType::Artists, cx)
            })),
        ))
        .child(render_overview_grid(
            overview
                .artists
                .iter()
                .take(3)
                .cloned()
                .map(|artist| {
                    cover_card::render_artist_card(
                        ArtistCoverCardProps {
                            name: artist.name,
                            cover_url: artist.cover_url,
                        },
                        CoverCardActions::default(),
                    )
                })
                .collect(),
            3,
            None,
            "暂无艺人结果",
            OVERVIEW_ARTIST_PLACEHOLDER_HEIGHT,
        ));
    let on_navigate_type_for_albums = on_navigate_type.clone();
    let albums = div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .flex_col()
        .gap_4()
        .child(section_header(
            "专辑",
            Some(Rc::new(move |cx| {
                on_navigate_type_for_albums(SearchRouteType::Albums, cx)
            })),
        ))
        .child(render_overview_grid(
            overview
                .albums
                .iter()
                .take(3)
                .cloned()
                .map(|album| {
                    cover_card::render_playlist_card(
                        PlaylistCoverCardProps {
                            title: album.name,
                            subtitle: album.artist_name,
                            cover_url: album.cover_url,
                            cover_height: px(PLAYLIST_CARD_HEIGHT),
                        },
                        CoverCardActions::default(),
                    )
                })
                .collect(),
            3,
            None,
            "暂无专辑结果",
            OVERVIEW_CARD_PLACEHOLDER_HEIGHT,
        ));
    let track_rows = overview
        .tracks
        .iter()
        .cloned()
        .map(|song| {
            let song_for_play = song.clone();
            let song_for_enqueue = song.clone();
            let on_play_song = on_play_song.clone();
            let on_enqueue_song = on_enqueue_song.clone();
            render_short_track_item(
                song,
                move |cx| on_play_song(song_for_play.clone(), cx),
                move |cx| on_enqueue_song(song_for_enqueue.clone(), cx),
            )
        })
        .collect::<Vec<_>>();
    let playlist_cards = overview
        .playlists
        .iter()
        .take(12)
        .cloned()
        .map(|playlist| {
            let playlist_id = playlist.id;
            let on_open_playlist = on_open_playlist.clone();
            cover_card::render_playlist_card(
                PlaylistCoverCardProps {
                    title: playlist.name,
                    subtitle: playlist.creator_name,
                    cover_url: playlist.cover_url,
                    cover_height: px(PLAYLIST_CARD_HEIGHT),
                },
                CoverCardActions {
                    on_open: Some(Rc::new(move |cx| on_open_playlist(playlist_id, cx))),
                },
            )
        })
        .collect::<Vec<_>>();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_10()
        .child(
            div()
                .w_full()
                .flex()
                .items_start()
                .gap(px(48.))
                .child(artists)
                .child(albums),
        )
        .child({
            let on_navigate_type = on_navigate_type.clone();
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_4()
                .child(section_header(
                    "歌曲",
                    Some(Rc::new(move |cx| {
                        on_navigate_type(SearchRouteType::Tracks, cx)
                    })),
                ))
                .child(render_overview_grid(
                    track_rows,
                    SHORT_TRACK_COLUMNS,
                    Some(SHORT_TRACK_GRID_GAP),
                    "暂无歌曲结果",
                    OVERVIEW_TRACK_PLACEHOLDER_HEIGHT,
                ))
        })
        .child({
            let on_navigate_type = on_navigate_type.clone();
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_4()
                .child(section_header(
                    "歌单",
                    Some(Rc::new(move |cx| {
                        on_navigate_type(SearchRouteType::Playlists, cx)
                    })),
                ))
                .child(render_overview_grid(
                    playlist_cards,
                    PLAYLIST_GRID_COLUMNS,
                    None,
                    "暂无歌单结果",
                    OVERVIEW_CARD_PLACEHOLDER_HEIGHT,
                ))
        })
        .into_any_element()
}

fn render_type_page(
    route_type: SearchRouteType,
    search_state: &crate::entity::pages::SearchPageState,
    current_playing_track_id: Option<i64>,
    on_play_song: PlaySongHandler,
    on_enqueue_song: EnqueueSongHandler,
    on_open_playlist: PlaylistOpenHandler,
    on_load_more: NavigateHandler,
) -> AnyElement {
    match route_type {
        SearchRouteType::Artists => render_collection_page(
            route_type,
            &search_state.artists,
            render_grid(
                search_state
                    .artists
                    .items
                    .data
                    .iter()
                    .cloned()
                    .map(|artist| {
                        cover_card::render_artist_card(
                            ArtistCoverCardProps {
                                name: artist.name,
                                cover_url: artist.cover_url,
                            },
                            CoverCardActions::default(),
                        )
                    })
                    .collect(),
                PLAYLIST_GRID_COLUMNS,
                "暂无艺人结果",
            ),
            on_load_more,
        ),
        SearchRouteType::Albums => render_collection_page(
            route_type,
            &search_state.albums,
            render_grid(
                search_state
                    .albums
                    .items
                    .data
                    .iter()
                    .cloned()
                    .map(|album| {
                        cover_card::render_playlist_card(
                            PlaylistCoverCardProps {
                                title: album.name,
                                subtitle: album.artist_name,
                                cover_url: album.cover_url,
                                cover_height: px(PLAYLIST_CARD_HEIGHT),
                            },
                            CoverCardActions::default(),
                        )
                    })
                    .collect(),
                SEARCH_TYPE_CARD_COLUMNS,
                "暂无专辑结果",
            ),
            on_load_more,
        ),
        SearchRouteType::Tracks => render_collection_page(
            route_type,
            &search_state.tracks,
            {
                let rows = search_state
                    .tracks
                    .items
                    .data
                    .iter()
                    .cloned()
                    .map(|song| {
                        let is_playing = current_playing_track_id == Some(song.id);
                        let song_for_play = song.clone();
                        let song_for_enqueue = song.clone();
                        let on_play_song = on_play_song.clone();
                        let on_enqueue_song = on_enqueue_song.clone();
                        render_track_row(
                            song,
                            is_playing,
                            move |cx| on_play_song(song_for_play.clone(), cx),
                            move |cx| on_enqueue_song(song_for_enqueue.clone(), cx),
                        )
                    })
                    .collect::<Vec<_>>();
                if rows.is_empty() {
                    common::empty_card("暂无歌曲结果")
                } else {
                    common::stacked_rows(rows, px(8.))
                }
            },
            on_load_more,
        ),
        SearchRouteType::Playlists => render_collection_page(
            route_type,
            &search_state.playlists,
            render_grid(
                search_state
                    .playlists
                    .items
                    .data
                    .iter()
                    .cloned()
                    .map(|playlist| {
                        let playlist_id = playlist.id;
                        let on_open_playlist = on_open_playlist.clone();
                        cover_card::render_playlist_card(
                            PlaylistCoverCardProps {
                                title: playlist.name,
                                subtitle: playlist.creator_name,
                                cover_url: playlist.cover_url,
                                cover_height: px(PLAYLIST_CARD_HEIGHT),
                            },
                            CoverCardActions {
                                on_open: Some(Rc::new(move |cx| on_open_playlist(playlist_id, cx))),
                            },
                        )
                    })
                    .collect(),
                SEARCH_TYPE_CARD_COLUMNS,
                "暂无歌单结果",
            ),
            on_load_more,
        ),
    }
}

fn render_collection_page<T>(
    route_type: SearchRouteType,
    state: &SearchCollectionState<T>,
    body: AnyElement,
    on_load_more: NavigateHandler,
) -> AnyElement {
    let status = common::status_banner(
        state.items.loading,
        state.items.error.as_deref(),
        format!("正在搜索{}...", route_type.label()),
        format!("{}搜索失败", route_type.label()),
    );
    let load_more = if state.has_more && !state.items.loading && state.items.error.is_none() {
        Some(
            div()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    button::pill_base("加载更多")
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_load_more(cx)),
                )
                .into_any_element(),
        )
    } else {
        None
    };

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_5()
        .child(status)
        .child(body)
        .children(load_more)
        .into_any_element()
}
