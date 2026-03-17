use std::sync::Arc;

use nekowg::{
    AnyElement, App, Context, Entity, FontWeight, MouseButton, ObjectFit, Render, Subscription,
    TextAlign, Window, div, img, linear_color_stop, linear_gradient, prelude::*, px, relative, rgb,
    rgba,
};

use crate::action::library_actions;
use crate::entity::app::HomeArtistLanguage;
use crate::entity::pages::DataState;
use crate::entity::player_controller::{PlayerController, QueueTrackInput};
use crate::entity::runtime::AppRuntime;
use crate::entity::services::{auth, pages};
use crate::router::{self, RouterState};
use crate::view::common;
use crate::{
    component::{
        button,
        icon::{self, IconName},
        theme,
    },
    util::url::image_resize_url,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HomeSessionKey {
    user_id: Option<i64>,
    has_user_token: bool,
    has_guest_token: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomePlaylistCard {
    pub id: i64,
    pub name: String,
    pub subtitle: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeArtistCard {
    pub name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HomePageSnapshot {
    pub loading: bool,
    pub error: Option<String>,
    pub daily_card: HomePlaylistCard,
    pub daily_first_track_id: Option<i64>,
    pub fm_card: HomePlaylistCard,
    pub fm_track: Option<library_actions::FmTrackItem>,
    pub playlists: Vec<HomePlaylistCard>,
    pub artists: Vec<HomeArtistCard>,
    pub albums: Vec<HomePlaylistCard>,
    pub toplists: Vec<HomePlaylistCard>,
}

impl HomePageSnapshot {
    pub fn from_states(
        recommend_playlists: &DataState<Vec<library_actions::LibraryPlaylistItem>>,
        recommend_artists: &DataState<Vec<library_actions::ArtistItem>>,
        new_albums: &DataState<Vec<library_actions::AlbumItem>>,
        toplists: &DataState<Vec<library_actions::ToplistItem>>,
        daily_tracks: &DataState<Vec<library_actions::DailyTrackItem>>,
        personal_fm: &DataState<Option<library_actions::FmTrackItem>>,
    ) -> Self {
        let loading = recommend_playlists.loading
            || recommend_artists.loading
            || new_albums.loading
            || toplists.loading
            || daily_tracks.loading
            || personal_fm.loading;
        let error = recommend_playlists
            .error
            .clone()
            .or(recommend_artists.error.clone())
            .or(new_albums.error.clone())
            .or(toplists.error.clone())
            .or(daily_tracks.error.clone())
            .or(personal_fm.error.clone());
        let daily_card = HomePlaylistCard {
            id: 0,
            name: "每日推荐".to_string(),
            subtitle: "根据你的口味更新".to_string(),
            cover_url: daily_tracks
                .data
                .first()
                .and_then(|track| track.cover_url.clone()),
        };
        let fm_card = personal_fm
            .data
            .as_ref()
            .map(|track| HomePlaylistCard {
                id: track.id,
                name: track.name.clone(),
                subtitle: track.artists.clone(),
                cover_url: track.cover_url.clone(),
            })
            .unwrap_or(HomePlaylistCard {
                id: 0,
                name: "私人 FM".to_string(),
                subtitle: "连续播放你可能喜欢的音乐".to_string(),
                cover_url: None,
            });

        Self {
            loading,
            error,
            daily_card,
            daily_first_track_id: daily_tracks.data.first().map(|track| track.id),
            fm_card,
            fm_track: personal_fm.data.clone(),
            playlists: recommend_playlists
                .data
                .iter()
                .take(10)
                .map(|item| HomePlaylistCard {
                    id: item.id,
                    name: item.name.clone(),
                    subtitle: item.creator_name.clone(),
                    cover_url: item.cover_url.clone(),
                })
                .collect(),
            artists: recommend_artists
                .data
                .iter()
                .take(6)
                .map(|artist| HomeArtistCard {
                    name: artist.name.clone(),
                    cover_url: artist.cover_url.clone(),
                })
                .collect(),
            albums: new_albums
                .data
                .iter()
                .take(10)
                .map(|album| HomePlaylistCard {
                    id: album.id,
                    name: album.name.clone(),
                    subtitle: album.artist_name.clone(),
                    cover_url: album.cover_url.clone(),
                })
                .collect(),
            toplists: toplists
                .data
                .iter()
                .take(10)
                .map(|list| HomePlaylistCard {
                    id: list.id,
                    name: list.name.clone(),
                    subtitle: list.update_frequency.clone(),
                    cover_url: list.cover_url.clone(),
                })
                .collect(),
        }
    }
}

pub fn daily_featured_card(
    item: HomePlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
    on_play: impl Fn(&mut App) + 'static,
) -> AnyElement {
    featured_daily_card(item, on_open, on_play)
}

pub fn fm_featured_card(
    item: HomePlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
) -> AnyElement {
    featured_fm_card(item, on_open)
}

pub fn artist_card(
    name: String,
    cover_url: Option<String>,
    on_open: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let mut avatar = div()
        .w_full()
        .relative()
        .pb(relative(1.0))
        .rounded_full()
        .overflow_hidden();

    if let Some(url) = cover_url.as_deref() {
        avatar = avatar.child(
            img(image_resize_url(url, "256y256"))
                .id(format!("home-artist-{url}"))
                .absolute()
                .left(px(0.))
                .top(px(0.))
                .right(px(0.))
                .bottom(px(0.))
                .size_full()
                .object_fit(ObjectFit::Cover)
                .rounded_full(),
        );
    } else {
        avatar = avatar.bg(rgb(0x3B3B3B));
    }

    div()
        .w_full()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx))
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .child(avatar)
                .child(
                    div()
                        .mt(px(12.))
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_align(TextAlign::Center)
                        .overflow_hidden()
                        .child(name),
                ),
        )
        .into_any_element()
}

fn featured_daily_card(
    item: HomePlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
    on_play: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let cover = item.cover_url.clone();
    div()
        .w_full()
        .h(px(198.))
        .rounded_xl()
        .overflow_hidden()
        .cursor_pointer()
        .relative()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx))
        .child(match cover {
            Some(url) => img(image_resize_url(&url, "512y512"))
                .id(format!("home-daily-featured-{}", &url))
                .w_full()
                .h_full()
                .rounded_xl()
                .into_any_element(),
            None => div()
                .w_full()
                .h_full()
                .rounded_xl()
                .bg(rgb(0x3B3B3B))
                .into_any_element(),
        })
        .child(
            div()
                .absolute()
                .left(px(0.))
                .top(px(0.))
                .right(px(0.))
                .bottom(px(0.))
                .h_full()
                .px(px(24.))
                .py(px(20.))
                .bg(rgba(theme::with_alpha(0x000000, 0x2E)))
                .flex()
                .items_center()
                .child(
                    div()
                        .w(px(148.))
                        .h(px(148.))
                        .text_size(px(64.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(theme::COLOR_TEXT_DARK))
                        .grid()
                        .grid_cols(2)
                        .justify_center()
                        .items_center()
                        .line_height(px(52.))
                        .children(["每", "日", "推", "荐"]),
                ),
        )
        .child(
            div()
                .absolute()
                .right(px(20.))
                .bottom(px(18.))
                .size(px(44.))
                .rounded_full()
                .bg(rgba(theme::with_alpha(0xFFFFFF, 0x38)))
                .flex()
                .justify_center()
                .items_center()
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    on_play(cx);
                })
                .child(icon::render(IconName::Play, 18.0, theme::COLOR_TEXT_DARK)),
        )
        .into_any_element()
}

fn featured_fm_card(item: HomePlaylistCard, on_open: impl Fn(&mut App) + 'static) -> AnyElement {
    let cover = item.cover_url.clone();
    let mut card = div()
        .w_full()
        .h(px(198.))
        .rounded_xl()
        .overflow_hidden()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx));

    if let Some(url) = cover.as_deref() {
        card = card.bg(gradient_from_seed(url));
    } else {
        card = card.bg(rgb(0x8D8D8D));
    }

    card.child(
        div()
            .size_full()
            .px(px(16.))
            .py(px(14.))
            .flex()
            .gap(px(16.))
            .child(match cover {
                Some(url) => img(image_resize_url(&url, "256y256"))
                    .id(format!("home-fm-featured-{}", &url))
                    .w(px(169.))
                    .h(px(169.))
                    .flex_shrink_0()
                    .rounded_lg()
                    .overflow_hidden()
                    .into_any_element(),
                None => div()
                    .w(px(169.))
                    .h(px(169.))
                    .flex_shrink_0()
                    .rounded_lg()
                    .bg(rgb(0x6F6F6F))
                    .into_any_element(),
            })
            .child(
                div()
                    .flex_grow()
                    .min_w(px(0.))
                    .h_full()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .pt(px(4.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .w_full()
                                    .text_size(px(28.))
                                    .line_height(px(28.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                                    .truncate()
                                    .child(item.name),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(15.))
                                    .text_color(rgba(theme::with_alpha(0xFFFFFF, 0xA8)))
                                    .overflow_hidden()
                                    .child(item.subtitle),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_between()
                            .items_end()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .child(icon_button(IconName::ThumbsDown))
                                    .child(icon_button(IconName::Play))
                                    .child(icon_button(IconName::Next)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .opacity(0.38)
                                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                                    .child(icon::render(IconName::Fm, 16.0, theme::COLOR_TEXT_DARK))
                                    .child("私人FM"),
                            ),
                    ),
            ),
    )
    .into_any_element()
}

fn icon_button(icon_name: IconName) -> AnyElement {
    let style = button::ButtonStyle {
        padding: px(0.),
        margin: px(0.),
        radius: px(8.),
        base_bg: button::transparent_bg(),
        hover_bg: rgba(theme::with_alpha(0xFFFFFF, 0x18)),
        hover_duration_ms: 180,
    };

    button::icon_interactive(
        format!("home-fm-icon-{icon_name:?}"),
        button::icon_base(style).size(px(34.)).child(icon::render(
            icon_name,
            18.0,
            theme::COLOR_TEXT_DARK,
        )),
        style,
    )
    .into_any_element()
}

fn gradient_from_seed(seed: &str) -> nekowg::Background {
    let base = color_from_seed(seed);
    let darker = shift_color(base, -32);
    let lighter = shift_color(base, 28);
    linear_gradient(
        120.0,
        linear_color_stop(rgb(lighter), 0.0),
        linear_color_stop(rgb(darker), 1.0),
    )
}

fn color_from_seed(seed: &str) -> u32 {
    let mut hash = 2166136261u32;
    for byte in seed.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    let r = ((hash >> 16) & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = (hash & 0xFF) as u8;
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn shift_color(color: u32, delta: i16) -> u32 {
    let r = ((color >> 16) & 0xFF) as i16 + delta;
    let g = ((color >> 8) & 0xFF) as i16 + delta;
    let b = (color & 0xFF) as i16 + delta;
    let r = r.clamp(0, 255) as u32;
    let g = g.clamp(0, 255) as u32;
    let b = b.clamp(0, 255) as u32;
    (r << 16) | (g << 8) | b
}

pub fn playlist_card(item: HomePlaylistCard, on_open: impl Fn(&mut App) + 'static) -> AnyElement {
    let cover = item.cover_url.clone();
    div()
        .w_full()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx))
        .child(match cover {
            Some(url) => img(image_resize_url(&url, "256y256"))
                .id(format!("home-playlist-{}", &url))
                .w_full()
                .h(px(166.))
                .rounded_lg()
                .overflow_hidden()
                .into_any_element(),
            None => div()
                .w_full()
                .h(px(166.))
                .rounded_lg()
                .bg(rgb(0x3B3B3B))
                .into_any_element(),
        })
        .child(
            div()
                .mt(px(8.))
                .text_size(px(16.))
                .line_height(relative(1.2))
                .font_weight(FontWeight::BOLD)
                .overflow_hidden()
                .child(item.name),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(12.))
                .line_height(relative(1.2))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_SECONDARY))
                .overflow_hidden()
                .child(item.subtitle),
        )
        .into_any_element()
}

fn grid_section(rows: Vec<AnyElement>, empty_label: &'static str, columns: usize) -> AnyElement {
    if rows.is_empty() {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_CARD_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child(empty_label)
            .into_any_element()
    } else {
        rows.into_iter()
            .fold(
                div().w_full().grid().grid_cols(columns as u16).gap(px(18.)),
                |grid, item| grid.child(item),
            )
            .into_any_element()
    }
}

pub struct HomePageView {
    runtime: AppRuntime,
    player_controller: Entity<PlayerController>,
    observed_session_key: HomeSessionKey,
    observed_artist_language: HomeArtistLanguage,
    _subscriptions: Vec<Subscription>,
}

impl HomePageView {
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
        subscriptions.push(cx.observe(&runtime.app, |this, _, cx| {
            this.handle_app_change(cx);
        }));
        let observed_session_key = session_key(&runtime, cx);
        let observed_artist_language = runtime.app.read(cx).home_artist_language;
        let mut this = Self {
            runtime,
            player_controller,
            observed_session_key,
            observed_artist_language,
            _subscriptions: subscriptions,
        };
        if this.is_active(cx) {
            this.ensure_loaded(cx);
        }
        this
    }

    fn is_active(&self, cx: &mut Context<Self>) -> bool {
        router::current_path(cx).as_ref() == "/"
    }

    fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        self.load(false, cx);
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.load(true, cx);
    }

    fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let key = session_key(&self.runtime, cx);
        let changed = self.observed_session_key != key;
        self.observed_session_key = key;

        if !self.is_active(cx) {
            return;
        }

        if changed {
            self.reload(cx);
        } else {
            cx.notify();
        }
    }

    fn handle_app_change(&mut self, cx: &mut Context<Self>) {
        let language = self.runtime.app.read(cx).home_artist_language;
        let changed = self.observed_artist_language != language;
        self.observed_artist_language = language;

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
        let key = session_key_from_session(&session);
        if !key.has_guest_token {
            return;
        }

        let source = if key.has_user_token {
            crate::entity::pages::DataSource::User
        } else {
            crate::entity::pages::DataSource::Guest
        };
        let home = self.runtime.home.read(cx).clone();
        if !force {
            if home.recommend_playlists.loading {
                return;
            }
            if home.recommend_playlists.source == source
                && home.recommend_playlists.fetched_at_ms.is_some()
            {
                return;
            }
        }

        let Some(cookie) = crate::action::auth_actions::build_cookie_header(&session.auth_bundle)
        else {
            return;
        };
        let artist_language = self.runtime.app.read(cx).home_artist_language;

        self.runtime.home.update(cx, |home, cx| {
            home.recommend_playlists.begin(source);
            home.recommend_artists.begin(source);
            home.new_albums.begin(source);
            home.toplists.begin(source);
            if key.has_user_token {
                home.daily_tracks
                    .begin(crate::entity::pages::DataSource::User);
                home.personal_fm
                    .begin(crate::entity::pages::DataSource::User);
            } else {
                home.daily_tracks.clear();
                home.personal_fm.clear();
            }
            cx.notify();
        });

        let page = cx.entity();
        cx.spawn(async move |_, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    pages::fetch_home_payload(&cookie, key.has_user_token, artist_language)
                })
                .await;
            page.update(cx, |this, cx| {
                this.apply_home_load(key, artist_language, result, cx)
            });
        })
        .detach();
    }

    fn apply_home_load(
        &mut self,
        key: HomeSessionKey,
        artist_language: HomeArtistLanguage,
        result: Result<pages::HomeLoadResult, String>,
        cx: &mut Context<Self>,
    ) {
        if session_key(&self.runtime, cx) != key
            || self.runtime.app.read(cx).home_artist_language != artist_language
        {
            return;
        }

        self.runtime.home.update(cx, |home, cx| {
            match result {
                Ok(result) => {
                    home.recommend_playlists
                        .succeed(result.recommend_playlists, Some(result.fetched_at_ms));
                    home.recommend_artists
                        .succeed(result.recommend_artists, Some(result.fetched_at_ms));
                    home.new_albums
                        .succeed(result.new_albums, Some(result.fetched_at_ms));
                    home.toplists
                        .succeed(result.toplists, Some(result.fetched_at_ms));
                    if key.has_user_token {
                        home.daily_tracks
                            .succeed(result.daily_tracks, Some(result.fetched_at_ms));
                        home.personal_fm
                            .succeed(result.personal_fm, Some(result.fetched_at_ms));
                    } else {
                        home.daily_tracks.clear();
                        home.personal_fm.clear();
                    }
                }
                Err(err) => {
                    home.recommend_playlists.clear();
                    home.recommend_playlists.fail(err.clone());
                    home.recommend_artists.clear();
                    home.recommend_artists.fail(err.clone());
                    home.new_albums.clear();
                    home.new_albums.fail(err.clone());
                    home.toplists.clear();
                    home.toplists.fail(err.clone());
                    if key.has_user_token {
                        home.daily_tracks.clear();
                        home.daily_tracks.fail(err.clone());
                        home.personal_fm.clear();
                        home.personal_fm.fail(err);
                    } else {
                        home.daily_tracks.clear();
                        home.personal_fm.clear();
                    }
                }
            }
            cx.notify();
        });
    }

    fn open_daily(&mut self, cx: &mut Context<Self>) {
        if auth::has_user_token(&self.runtime, cx) {
            router::navigate(cx, "/daily/songs");
        } else {
            router::navigate(cx, "/login");
        }
    }

    fn play_daily(&mut self, track_id: Option<i64>, cx: &mut Context<Self>) {
        if !auth::has_user_token(&self.runtime, cx) {
            router::navigate(cx, "/login");
            return;
        }

        let tracks = self.runtime.home.read(cx).daily_tracks.data.clone();
        if tracks.is_empty() {
            return;
        }
        let start_index = track_id
            .and_then(|track_id| tracks.iter().position(|track| track.id == track_id))
            .unwrap_or(0);
        let queue = tracks
            .into_iter()
            .map(QueueTrackInput::from)
            .collect::<Vec<_>>();
        self.player_controller.update(cx, |player, cx| {
            player.replace_queue(queue, start_index, cx)
        });
    }

    fn open_fm(&mut self, track: Option<library_actions::FmTrackItem>, cx: &mut Context<Self>) {
        if !auth::has_user_token(&self.runtime, cx) {
            router::navigate(cx, "/login");
            return;
        }
        if let Some(track) = track {
            self.player_controller.update(cx, |player, cx| {
                player.enqueue_track(track.into(), true, cx)
            });
        } else {
            router::navigate(cx, "/library");
        }
    }
}

fn session_key(runtime: &AppRuntime, cx: &Context<HomePageView>) -> HomeSessionKey {
    runtime
        .session
        .read_with(cx, |session, _| session_key_from_session(session))
}

fn session_key_from_session(session: &crate::entity::session::SessionState) -> HomeSessionKey {
    HomeSessionKey {
        user_id: session.auth_user_id,
        has_user_token: session
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        has_guest_token: session
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            || session
                .auth_bundle
                .music_a
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
    }
}

impl Render for HomePageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let home = self.runtime.home.read(cx).clone();
        let snapshot = HomePageSnapshot::from_states(
            &home.recommend_playlists,
            &home.recommend_artists,
            &home.new_albums,
            &home.toplists,
            &home.daily_tracks,
            &home.personal_fm,
        );
        let page = cx.entity();
        let on_open_daily = {
            let page = page.clone();
            Arc::new(move |cx: &mut App| {
                page.update(cx, |this, cx| this.open_daily(cx));
            })
        };
        let on_play_daily = {
            let page = page.clone();
            Arc::new(move |track_id: Option<i64>, cx: &mut App| {
                page.update(cx, |this, cx| this.play_daily(track_id, cx));
            })
        };
        let on_open_fm = {
            let page = page.clone();
            Arc::new(
                move |track: Option<library_actions::FmTrackItem>, cx: &mut App| {
                    page.update(cx, |this, cx| this.open_fm(track.clone(), cx));
                },
            )
        };
        let on_open_playlist = Arc::new(move |playlist_id: i64, cx: &mut App| {
            page.update(cx, |_, cx| {
                router::navigate(cx, format!("/playlist/{playlist_id}"));
            });
        });

        let featured_rows = vec![
            {
                let on_open_daily = on_open_daily.clone();
                let on_play_daily = on_play_daily.clone();
                daily_featured_card(
                    snapshot.daily_card,
                    move |cx| on_open_daily(cx),
                    move |cx| on_play_daily(snapshot.daily_first_track_id, cx),
                )
            },
            {
                let on_open_fm = on_open_fm.clone();
                fm_featured_card(snapshot.fm_card, move |cx| {
                    on_open_fm(snapshot.fm_track.clone(), cx)
                })
            },
        ];
        let playlist_rows = snapshot
            .playlists
            .into_iter()
            .map(|item| {
                let playlist_id = item.id;
                let on_open_playlist = on_open_playlist.clone();
                playlist_card(item, move |cx| on_open_playlist(playlist_id, cx))
            })
            .collect();
        let artist_rows = snapshot
            .artists
            .into_iter()
            .map(|artist| artist_card(artist.name, artist.cover_url, move |_cx| {}))
            .collect();
        let album_rows = snapshot
            .albums
            .into_iter()
            .map(|item| playlist_card(item, move |_cx| {}))
            .collect();
        let toplist_rows = snapshot
            .toplists
            .into_iter()
            .map(|item| playlist_card(item, move |_cx| {}))
            .collect();

        let status = common::status_banner(
            snapshot.loading,
            snapshot.error.as_deref(),
            "加载中...",
            "加载失败",
        );

        let featured = if featured_rows.is_empty() {
            div()
                .w_full()
                .rounded_lg()
                .bg(rgb(theme::COLOR_CARD_DARK))
                .px_4()
                .py_3()
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child("暂无推荐")
                .into_any_element()
        } else {
            featured_rows
                .into_iter()
                .fold(
                    div().w_full().grid().grid_cols(2).gap(px(20.)),
                    |col, item| col.child(item),
                )
                .into_any_element()
        };

        let playlists = grid_section(playlist_rows, "暂无推荐歌单", 5);
        let artists = grid_section(artist_rows, "暂无推荐艺人", 6);
        let albums = grid_section(album_rows, "暂无新碟", 5);
        let toplists = grid_section(toplist_rows, "暂无榜单", 5);

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(28.))
            .child(div().w_full().mt(px(12.)).child(status))
            .child(
                div()
                    .w_full()
                    .mb(px(22.))
                    .text_size(px(26.))
                    .font_weight(FontWeight::BOLD)
                    .child("For You"),
            )
            .child(featured)
            .child(
                div()
                    .w_full()
                    .mt(px(36.))
                    .mb(px(14.))
                    .text_size(px(26.))
                    .font_weight(FontWeight::BOLD)
                    .child("推荐歌单"),
            )
            .child(playlists)
            .child(
                div()
                    .w_full()
                    .mt(px(40.))
                    .mb(px(14.))
                    .text_size(px(26.))
                    .font_weight(FontWeight::BOLD)
                    .child("推荐艺人"),
            )
            .child(artists)
            .child(
                div()
                    .w_full()
                    .mt(px(40.))
                    .mb(px(14.))
                    .text_size(px(26.))
                    .font_weight(FontWeight::BOLD)
                    .child("新碟上架"),
            )
            .child(albums)
            .child(
                div()
                    .w_full()
                    .mt(px(40.))
                    .mb(px(14.))
                    .text_size(px(26.))
                    .font_weight(FontWeight::BOLD)
                    .child("榜单"),
            )
            .child(toplists)
            .into_any_element()
    }
}
