use std::sync::Arc;

use nekowg::{AnyElement, App, FontWeight, div, prelude::*, px, rgb};

use crate::component::{page, theme};
use crate::page::daily_tracks::models::DailyTracksPageSnapshot;
use crate::page::playlist::{self, PlaylistTrackRow};

pub(crate) type TrackActionHandler = Arc<dyn Fn(PlaylistTrackRow, &mut App)>;
pub(crate) type ReplaceDailyQueueHandler = Arc<dyn Fn(Option<i64>, &mut App)>;

pub(crate) fn render_daily_tracks_page(
    snapshot: DailyTracksPageSnapshot,
    on_play_track: TrackActionHandler,
    on_enqueue_track: TrackActionHandler,
    on_replace_queue: ReplaceDailyQueueHandler,
) -> AnyElement {
    let rows = snapshot
        .tracks
        .into_iter()
        .enumerate()
        .map(|(index, track)| {
            let is_playing = snapshot.current_playing_track_id == Some(track.id);
            let play_track = track.clone();
            let queue_track = track.clone();
            let on_play_track = on_play_track.clone();
            let on_enqueue_track = on_enqueue_track.clone();
            playlist::track_row(
                format!("daily-tracks:row:{index}:track:{}", track.id),
                track,
                is_playing,
                move |cx| on_play_track(play_track.clone(), cx),
                move |cx| on_enqueue_track(queue_track.clone(), cx),
            )
        })
        .collect::<Vec<_>>();
    let action = snapshot.first_track_id.map(|track_id| {
        let on_replace_queue = on_replace_queue.clone();
        crate::component::button::primary_pill("替换队列并播放")
            .on_mouse_down(nekowg::MouseButton::Left, move |_, _, cx| {
                on_replace_queue(Some(track_id), cx);
            })
            .into_any_element()
    });
    let status = page::status_banner(
        snapshot.loading,
        snapshot.error.as_deref(),
        "加载中...",
        "加载失败",
    );
    let list = if rows.is_empty() {
        page::empty_card("暂无歌曲")
    } else {
        page::stacked_rows(rows, px(8.))
    };
    let header_content = div()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(42.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_TEXT_DARK))
                .child("每日歌曲推荐"),
        )
        .child(
            div()
                .text_size(px(16.))
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child("根据你的音乐口味生成 · 每天 6:00 更新"),
        );
    let header = if let Some(action) = action {
        div()
            .w_full()
            .flex()
            .items_end()
            .justify_between()
            .child(header_content)
            .child(action)
    } else {
        header_content
    };

    div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(28.))
        .gap_4()
        .child(header)
        .child(status)
        .child(list)
        .into_any_element()
}
