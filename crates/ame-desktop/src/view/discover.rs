use gpui::{AnyElement, App, FontWeight, MouseButton, div, img, prelude::*, px, rgb};

use crate::component::{button, theme};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverPlaylistCard {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

pub fn playlist_card(
    item: DiscoverPlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
) -> AnyElement {
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
                .items_center()
                .gap(px(12.))
                .child(match item.cover_url.clone() {
                    Some(url) => img(url)
                        .size(px(58.))
                        .rounded_md()
                        .overflow_hidden()
                        .into_any_element(),
                    None => div()
                        .size(px(58.))
                        .rounded_md()
                        .bg(rgb(0x3B3B3B))
                        .into_any_element(),
                })
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
        )
        .child(
            button::pill_base("打开").on_mouse_down(MouseButton::Left, move |_, _, cx| {
                on_open(cx);
            }),
        )
        .into_any_element()
}

pub fn render(loading: bool, error: Option<&str>, rows: Vec<AnyElement>) -> AnyElement {
    let status = if let Some(error) = error {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child(format!("加载失败: {error}"))
            .into_any_element()
    } else if loading {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("加载中...")
            .into_any_element()
    } else {
        div().into_any_element()
    };

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
        rows.into_iter()
            .fold(div().w_full().flex().flex_col().gap_2(), |list, row| {
                list.child(row)
            })
            .into_any_element()
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
        .into_any_element()
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
