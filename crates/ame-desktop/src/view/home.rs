use nekowg::{AnyElement, App, FontWeight, MouseButton, div, img, prelude::*, px, rgb, rgba};

use crate::view::common;
use crate::{
    component::{
        button,
        icon::{self, IconName},
        theme,
    },
    util::url::image_resize_url,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomePlaylistCard {
    pub id: i64,
    pub name: String,
    pub subtitle: String,
    pub cover_url: Option<String>,
}

pub fn daily_featured_card(
    item: HomePlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
) -> AnyElement {
    featured_daily_card(item, on_open)
}

pub fn fm_featured_card(
    item: HomePlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
) -> AnyElement {
    featured_fm_card(item, on_open)
}

fn featured_daily_card(item: HomePlaylistCard, on_open: impl Fn(&mut App) + 'static) -> AnyElement {
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
            Some(url) => img(image_resize_url(&url, "256y256"))
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
                .child(icon::render(IconName::Play, 18.0, theme::COLOR_TEXT_DARK)),
        )
        .into_any_element()
}

fn featured_fm_card(item: HomePlaylistCard, on_open: impl Fn(&mut App) + 'static) -> AnyElement {
    let cover = item.cover_url.clone();
    div()
        .w_full()
        .h(px(198.))
        .rounded_xl()
        .overflow_hidden()
        .cursor_pointer()
        .bg(rgb(0x8D8D8D))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx))
        .child(
            div()
                .size_full()
                .px(px(20.))
                .py(px(18.))
                .flex()
                .gap(px(16.))
                .child(match cover {
                    Some(url) => img(image_resize_url(&url, "256y256"))
                        .id(format!("home-fm-featured-{}", &url))
                        .w(px(162.))
                        .h(px(162.))
                        .rounded_lg()
                        .overflow_hidden()
                        .into_any_element(),
                    None => div()
                        .w(px(162.))
                        .h(px(162.))
                        .rounded_lg()
                        .bg(rgb(0x6F6F6F))
                        .into_any_element(),
                })
                .child(
                    div()
                        .flex_grow()
                        .h_full()
                        .flex()
                        .flex_col()
                        .justify_between()
                        .child(
                            div()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .text_size(px(50.))
                                        .line_height(px(44.))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(theme::COLOR_TEXT_DARK))
                                        .overflow_hidden()
                                        .child(item.name),
                                )
                                .child(
                                    div()
                                        .mt(px(4.))
                                        .text_size(px(16.))
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
                                        .text_size(px(18.))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(rgb(theme::COLOR_SECONDARY))
                                        .child(icon::render(
                                            IconName::Fm,
                                            16.0,
                                            theme::COLOR_SECONDARY,
                                        ))
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
                .font_weight(FontWeight::BOLD)
                .overflow_hidden()
                .child(item.name),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(13.))
                .text_color(rgb(theme::COLOR_SECONDARY))
                .overflow_hidden()
                .child(item.subtitle),
        )
        .into_any_element()
}

pub fn render(
    loading: bool,
    error: Option<&str>,
    featured_rows: Vec<AnyElement>,
    playlist_rows: Vec<AnyElement>,
) -> AnyElement {
    let status = common::status_banner(loading, error, "加载中...", "加载失败");

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

    let playlists = if playlist_rows.is_empty() {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_CARD_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("暂无推荐歌单")
            .into_any_element()
    } else {
        playlist_rows
            .into_iter()
            .fold(
                div().w_full().grid().grid_cols(5).gap(px(18.)),
                |grid, item| grid.child(item),
            )
            .into_any_element()
    };

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
                .mt(px(32.))
                .mb(px(18.))
                .text_size(px(26.))
                .font_weight(FontWeight::BOLD)
                .child("推荐歌单"),
        )
        .child(playlists)
        .into_any_element()
}
