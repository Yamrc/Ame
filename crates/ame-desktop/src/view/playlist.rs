use std::sync::Arc;

use nekowg::{AnyElement, App, FontWeight, div, prelude::*, px, rgb};
use serde::{Deserialize, Serialize};

use crate::component::track_item::{self, TrackItemActions, TrackItemProps};
use crate::component::theme;
use crate::view::common;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistTrackRow {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
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
    is_playing: bool,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
) -> AnyElement {
    track_item::render(
        TrackItemProps {
            id: item.id,
            title: item.name,
            alias: item.alias,
            artists: item.artists,
            album: item.album,
            duration_ms: item.duration_ms,
            cover_url: item.cover_url,
            show_cover: true,
            is_playing,
        },
        TrackItemActions {
            on_play: Some(Arc::new(on_play)),
            on_enqueue: Some(Arc::new(on_enqueue)),
            ..TrackItemActions::default()
        },
    )
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
