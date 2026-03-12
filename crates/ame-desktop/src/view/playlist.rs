use nekowg::{AnyElement, App, FontWeight, MouseButton, div, prelude::*, px, rgb};
use serde::{Deserialize, Serialize};

use crate::component::button;
use crate::component::theme;
use crate::view::common;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistTrackRow {
    pub id: i64,
    pub name: String,
    pub artists: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistPage {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub tracks: Vec<PlaylistTrackRow>,
}

pub fn track_row(
    item: PlaylistTrackRow,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
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
                        .child(item.artists),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(button::pill_base("播放").on_mouse_down(
                    MouseButton::Left,
                    move |_, _, cx| {
                        on_play(cx);
                    },
                ))
                .child(button::pill_base("入队").on_mouse_down(
                    MouseButton::Left,
                    move |_, _, cx| {
                        on_enqueue(cx);
                    },
                )),
        )
        .into_any_element()
}

pub fn render(
    playlist_id: &str,
    loading: bool,
    error: Option<&str>,
    playlist: Option<&PlaylistPage>,
    track_list: Option<AnyElement>,
    replace_queue_button: Option<AnyElement>,
) -> AnyElement {
    let title = playlist
        .map(|item| item.name.clone())
        .unwrap_or_else(|| format!("歌单 #{playlist_id}"));
    let subtitle = playlist
        .map(|item| format!("{} 首 · by {}", item.track_count, item.creator_name))
        .unwrap_or_else(|| "待加载".to_string());

    div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(28.))
        .gap_5()
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(38.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(theme::COLOR_TEXT_DARK))
                        .child(title),
                )
                .child(replace_queue_button.unwrap_or_else(|| div().into_any_element())),
        )
        .child(
            div()
                .text_size(px(16.))
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child(subtitle),
        )
        .child(common::status_banner(
            loading,
            error,
            "加载中...",
            "加载失败",
        ))
        .child(div().w_full().child(if let Some(track_list) = track_list {
            track_list
        } else {
            common::empty_card("暂无歌曲")
        }))
        .into_any_element()
}
