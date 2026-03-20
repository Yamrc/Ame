mod liked;

use std::rc::Rc;
use std::sync::Arc;

use nekowg::{AnyElement, App, FontWeight, MouseButton, div, img, prelude::*, px, relative, rgb};

use crate::component::playlist_card::{self, PlaylistCardActions, PlaylistCardProps};
use crate::component::{button, page, theme};
use crate::domain::library::PlaylistTrackItem;
use crate::page::library::models::{LibraryPlaylistCard, LibraryTab};
use crate::util::url::image_resize_url;

use self::liked::{empty_liked_card, liked_card, liked_preview_list};

const PREVIEW_COLS: usize = 3;
const PREVIEW_MAX: usize = 12;
const PREVIEW_ROW_HEIGHT: f32 = 52.0;
const PREVIEW_ROW_GAP: f32 = 8.0;
const PLAYLIST_GRID_COLUMNS: usize = 5;

pub(crate) type PreviewPlayHandler = Arc<dyn Fn(PlaylistTrackItem, &mut App)>;
pub(crate) type PlaylistActionHandler = Rc<dyn Fn(i64, &mut App)>;
pub(crate) type TabActionHandler = Arc<dyn Fn(&mut App)>;

pub(crate) struct LibrarySectionsRender<'a> {
    pub loading: bool,
    pub error: Option<&'a str>,
    pub liked_playlist: Option<&'a LibraryPlaylistCard>,
    pub liked_tracks: &'a [PlaylistTrackItem],
    pub liked_lyric_lines: &'a [String],
    pub created_playlists: &'a [LibraryPlaylistCard],
    pub collected_playlists: &'a [LibraryPlaylistCard],
    pub followed_playlists: &'a [LibraryPlaylistCard],
    pub active_tab: LibraryTab,
    pub title: &'a str,
    pub user_avatar: Option<&'a str>,
}

pub(crate) fn render_library_sections(
    view: LibrarySectionsRender<'_>,
    on_open_playlist: PlaylistActionHandler,
    on_replace_queue_from_playlist: PlaylistActionHandler,
    on_preview_play: PreviewPlayHandler,
    on_tab_created: TabActionHandler,
    on_tab_collected: TabActionHandler,
    on_tab_followed: TabActionHandler,
) -> AnyElement {
    let liked_tracks = view.liked_tracks;
    let liked_lyric_lines = view.liked_lyric_lines;
    let active_tab = view.active_tab;
    let title = view.title;
    let user_avatar = view.user_avatar;
    let preview_count = liked_tracks.len().min(PREVIEW_MAX);
    let preview_rows = preview_count.div_ceil(PREVIEW_COLS).max(2);
    let preview_height = preview_rows as f32 * PREVIEW_ROW_HEIGHT
        + (preview_rows.saturating_sub(1) as f32) * PREVIEW_ROW_GAP;
    let preview_min_height = px(preview_height);

    let liked_cover_card = view.liked_playlist.cloned().map(|item| {
        let playlist_id = item.id;
        let on_open_playlist = on_open_playlist.clone();
        let on_replace_queue_from_playlist = on_replace_queue_from_playlist.clone();
        liked_card(
            item,
            liked_lyric_lines,
            preview_min_height,
            move |cx| on_open_playlist(playlist_id, cx),
            move |cx| on_replace_queue_from_playlist(playlist_id, cx),
        )
    });
    let created_cards =
        build_playlist_cards(view.created_playlists.iter(), on_open_playlist.clone());
    let collected_cards =
        build_playlist_cards(view.collected_playlists.iter(), on_open_playlist.clone());
    let followed_cards =
        build_playlist_cards(view.followed_playlists.iter(), on_open_playlist.clone());

    let status = page::status_banner(view.loading, view.error, "加载中...", "加载失败");
    let liked_cover_card = liked_cover_card.unwrap_or_else(|| empty_liked_card(preview_min_height));
    let liked_preview = liked_preview_list(
        liked_tracks,
        PREVIEW_ROW_HEIGHT,
        PREVIEW_ROW_GAP,
        on_preview_play,
    );
    let header = build_header(title, user_avatar);

    let tabs = render_tabs(
        active_tab,
        on_tab_created,
        on_tab_collected,
        on_tab_followed,
    );
    let panel = match active_tab {
        LibraryTab::Created => page::grid_or_empty(
            created_cards,
            PLAYLIST_GRID_COLUMNS,
            px(18.),
            "暂无创建歌单",
        ),
        LibraryTab::Collected => page::grid_or_empty(
            collected_cards,
            PLAYLIST_GRID_COLUMNS,
            px(18.),
            "暂无收藏歌单",
        ),
        LibraryTab::Followed => {
            page::grid_or_empty(followed_cards, PLAYLIST_GRID_COLUMNS, px(18.), "暂无关注")
        }
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
                .child(liked_cover_card)
                .child(div().w(relative(0.671)).ml(px(36.)).child(liked_preview)),
        )
        .child(div().w_full().mt(px(20.)).child(status))
        .child(div().w_full().mt(px(20.)).child(tabs))
        .child(div().w_full().mt(px(16.)).child(panel))
        .into_any_element()
}

fn build_header(title: &str, user_avatar: Option<&str>) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .child(match user_avatar {
            Some(url) => img(image_resize_url(url, "96y96"))
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
                .child(title.to_string()),
        )
        .into_any_element()
}

fn render_tabs(
    active_tab: LibraryTab,
    on_tab_created: TabActionHandler,
    on_tab_collected: TabActionHandler,
    on_tab_followed: TabActionHandler,
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

fn build_playlist_cards(
    playlists: impl Iterator<Item = impl std::borrow::Borrow<LibraryPlaylistCard>>,
    on_open_playlist: PlaylistActionHandler,
) -> Vec<AnyElement> {
    playlists
        .map(|item| item.borrow().clone())
        .map(|item| {
            let playlist_id = item.id;
            let on_open_playlist = on_open_playlist.clone();
            playlist_card::render(
                PlaylistCardProps::standard(
                    item.name,
                    playlist_card::subtitle_with_count(Some(item.track_count), &item.creator_name),
                    item.cover_url,
                ),
                PlaylistCardActions {
                    on_open: Some(Rc::new(move |cx| on_open_playlist(playlist_id, cx))),
                },
            )
        })
        .collect()
}
