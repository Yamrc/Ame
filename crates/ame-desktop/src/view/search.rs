use nekowg::{AnyElement, App, FontWeight, div, prelude::*, px, rgb};

use crate::component::track_item::{self, TrackItemActions, TrackItemProps};
use crate::component::theme;
use crate::view::common;

#[derive(Debug, Clone)]
pub struct SearchSong {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
}

pub fn render_row(
    song: SearchSong,
    is_playing: bool,
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
            cover_url: None,
            show_cover: false,
            is_playing,
        },
        TrackItemActions {
            on_enqueue: Some(std::sync::Arc::new(on_enqueue)),
            ..TrackItemActions::default()
        },
    )
}

pub fn render(
    keyword: &str,
    loading: bool,
    error: Option<&str>,
    rows: Vec<AnyElement>,
) -> AnyElement {
    let title = if keyword.is_empty() {
        "搜索".to_string()
    } else {
        format!("搜索: {keyword}")
    };

    let status = common::status_banner(loading, error, "搜索中...", "搜索失败");

    let results = if rows.is_empty() {
        common::empty_card("暂无结果")
    } else {
        common::stacked_rows(rows, px(8.))
    };

    div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(28.))
        .gap_5()
        .child(
            div()
                .text_size(px(42.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_TEXT_DARK))
                .child(title),
        )
        .child(status)
        .child(results)
        .into_any_element()
}
