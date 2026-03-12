use nekowg::{
    AnyElement, App, FontWeight, MouseButton, div, img, prelude::*, px, relative, rgb,
};
use std::sync::Arc;

use crate::action::library_actions::PlaylistTrackItem;
use crate::component::{button, icon, theme};
use crate::util::url::image_resize_url;
use crate::view::common;
use nekowg::SharedString;

const PREVIEW_COLS: usize = 3;
const PREVIEW_MAX: usize = 12;
const PREVIEW_ROW_HEIGHT: f32 = 52.0;
const PREVIEW_ROW_GAP: f32 = 8.0;
type PreviewPlayHandler = Arc<dyn Fn(PlaylistTrackItem, &mut App)>;

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

pub struct LibraryViewModel {
    pub title: SharedString,
    pub user_avatar: Option<String>,
    pub loading: bool,
    pub error: Option<SharedString>,
    pub liked_card: Option<AnyElement>,
    pub liked_tracks: Vec<PlaylistTrackItem>,
    pub preview_min_height: nekowg::Pixels,
    pub active_tab: LibraryTab,
    pub created_rows: Vec<AnyElement>,
    pub collected_rows: Vec<AnyElement>,
    pub followed_rows: Vec<AnyElement>,
}

pub struct LibraryActions {
    pub on_tab_created: Arc<dyn Fn(&mut App)>,
    pub on_tab_collected: Arc<dyn Fn(&mut App)>,
    pub on_tab_followed: Arc<dyn Fn(&mut App)>,
    pub on_preview_play: PreviewPlayHandler,
}

pub fn playlist_row(item: LibraryPlaylistCard, on_open: impl Fn(&mut App) + 'static) -> AnyElement {
    div()
        .w_full()
        .rounded_lg()
        .bg(rgb(theme::COLOR_CARD_DARK))
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(18.))
                        .font_weight(FontWeight::BOLD)
                        .child(item.name),
                )
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(theme::COLOR_SECONDARY))
                        .child(format!(
                            "{} 首 · by {}",
                            item.track_count, item.creator_name
                        )),
                ),
        )
        .child(
            button::pill_base("打开").on_mouse_down(MouseButton::Left, move |_, _, cx| {
                on_open(cx);
            }),
        )
        .into_any_element()
}

pub fn render(mut model: LibraryViewModel, actions: LibraryActions) -> AnyElement {
    let status = common::status_banner(
        model.loading,
        model.error.as_ref().map(AsRef::as_ref),
        "加载中...",
        "加载失败",
    );
    let liked_card = model
        .liked_card
        .take()
        .unwrap_or_else(|| empty_liked_card(model.preview_min_height));
    let liked_preview = liked_preview_list(
        &model.liked_tracks,
        PREVIEW_ROW_HEIGHT,
        PREVIEW_ROW_GAP,
        actions.on_preview_play.clone(),
    );
    let header = build_header(&model.title, model.user_avatar.take());

    let tabs = render_tabs(
        model.active_tab,
        actions.on_tab_created,
        actions.on_tab_collected,
        actions.on_tab_followed,
    );
    let panel = match model.active_tab {
        LibraryTab::Created => render_tab_panel(model.created_rows, "暂无创建歌单"),
        LibraryTab::Collected => render_tab_panel(model.collected_rows, "暂无收藏歌单"),
        LibraryTab::Followed => render_tab_panel(model.followed_rows, "暂无关注"),
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
                .child(
                    div()
                        .w(relative(0.671))
                        .ml(px(36.))
                        .child(liked_preview),
                ),
        )
        .child(div().w_full().mt(px(20.)).child(status))
        .child(div().w_full().mt(px(20.)).child(tabs))
        .child(div().w_full().mt(px(16.)).child(panel))
        .into_any_element()
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

    let hover_style = button::ButtonStyle {
        padding: px(4.),
        margin: px(0.),
        radius: px(10.),
        base_bg: button::transparent_bg(),
        hover_bg: button::hover_bg(),
        hover_duration_ms: 160,
    };

    div()
        .overflow_hidden()
        .grid()
        .grid_cols(PREVIEW_COLS as u16)
        .gap(px(row_gap))
        .children(tracks.iter().take(PREVIEW_MAX).map(|track| {
            let cover = track.cover_url.clone();
            let track_for_play = track.clone();
            let on_play = on_play.clone();
            let row = div()
                .w_full()
                .h(px(row_height))
                .flex()
                .items_center()
                .gap(px(10.))
                .rounded(px(10.))
                .px(px(8.))
                .py(px(4.))
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                    if event.click_count >= 2 {
                        on_play(track_for_play.clone(), cx);
                    }
                })
                .child(match cover {
                    Some(url) => img(image_resize_url(&url, "64y64"))
                        .size(px(36.))
                        .rounded_md()
                        .overflow_hidden()
                        .flex_shrink_0()
                        .into_any_element(),
                    None => div()
                        .size(px(36.))
                        .rounded_md()
                        .bg(rgb(0x3B3B3B))
                        .flex_shrink_0()
                        .into_any_element(),
                })
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .child(
                            div()
                                .text_size(px(16.))
                                .font_weight(FontWeight::BOLD)
                                .overflow_hidden()
                                .truncate()
                                .child(track.name.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(rgb(theme::COLOR_SECONDARY))
                                .overflow_hidden()
                                .truncate()
                                .child(track.artists.clone()),
                        ),
                )
                ;

            button::icon_interactive(
                format!("library-liked-preview-{}", track.id),
                row,
                hover_style,
            )
            .into_any_element()
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

fn render_tab_panel(rows: Vec<AnyElement>, empty_label: &str) -> AnyElement {
    if rows.is_empty() {
        return common::empty_card(empty_label.to_string());
    }
    common::stacked_rows(rows, px(8.))
}
