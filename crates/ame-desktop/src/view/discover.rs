use std::sync::Arc;

use nekowg::{AnyElement, App, FontWeight, div, prelude::*, px, rgb};

use crate::component::button;
use crate::component::playlist_item::{self, PlaylistItemActions, PlaylistItemProps};
use crate::component::theme;
use crate::view::common;

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

pub fn render(loading: bool, error: Option<&str>, rows: Vec<AnyElement>) -> AnyElement {
    let status = common::status_banner(loading, error, "加载中...", "加载失败");

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
