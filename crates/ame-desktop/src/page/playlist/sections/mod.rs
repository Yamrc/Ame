mod track;

use std::rc::Rc;
use std::sync::Arc;

use nekowg::{
    AnyElement, App, FontWeight, ListSizingBehavior, ScrollHandle, div, prelude::*, px, rgb,
};

use crate::component::{button, page, theme, virtual_list};
use crate::page::playlist::models::{PlaylistPageSnapshot, PlaylistTrackRow};

pub(crate) use track::track_row;

pub(crate) type TrackActionHandler = Rc<dyn Fn(PlaylistTrackRow, &mut App)>;
pub(crate) type ReplaceQueueHandler = Rc<dyn Fn(i64, &mut App)>;

pub(crate) fn render_playlist_page(
    snapshot: PlaylistPageSnapshot,
    page_scroll_handle: &ScrollHandle,
    on_play_track: TrackActionHandler,
    on_enqueue_track: TrackActionHandler,
    on_replace_queue: ReplaceQueueHandler,
) -> AnyElement {
    let playlist_rows = snapshot.playlist.as_ref().and_then(|page| {
        if page.tracks.is_empty() {
            return None;
        }
        let playlist_id = page.id;
        let tracks = Arc::new(page.tracks.clone());
        let heights = Arc::new(vec![px(60.); tracks.len()]);
        let current_playing_track_id = snapshot.current_playing_track_id;
        let on_play_track = on_play_track.clone();
        let on_enqueue_track = on_enqueue_track.clone();
        let list = virtual_list::v_virtual_list(
            ("playlist-tracks", page.id.unsigned_abs() as usize),
            heights,
            move |visible_range, _, _| {
                visible_range
                    .map(|index| {
                        let track = tracks[index].clone();
                        let is_playing = current_playing_track_id == Some(track.id);
                        let play_track = track.clone();
                        let queue_track = track.clone();
                        let on_play_track = on_play_track.clone();
                        let on_enqueue_track = on_enqueue_track.clone();
                        nekowg::div().w_full().pb(px(4.)).child(track::track_row(
                            format!("playlist:{playlist_id}:row:{index}:track:{}", track.id),
                            track,
                            is_playing,
                            move |cx| on_play_track(play_track.clone(), cx),
                            move |cx| on_enqueue_track(queue_track.clone(), cx),
                        ))
                    })
                    .collect::<Vec<_>>()
            },
        )
        .with_external_viewport_scroll(page_scroll_handle)
        .with_sizing_behavior(ListSizingBehavior::Infer)
        .with_overscan(2)
        .w_full();
        Some(list.into_any_element())
    });
    let replace_queue_button = snapshot.playlist.as_ref().and_then(|page| {
        if page.tracks.is_empty() {
            return None;
        }
        let on_replace_queue = on_replace_queue.clone();
        let playlist_id = snapshot.playlist_id;
        Some(
            button::primary_pill("替换队列并播放")
                .on_mouse_down(nekowg::MouseButton::Left, move |_, _, cx| {
                    on_replace_queue(playlist_id, cx);
                })
                .into_any_element(),
        )
    });
    let title = snapshot
        .playlist
        .as_ref()
        .map(|item| item.name.clone())
        .unwrap_or_else(|| format!("歌单 #{}", snapshot.playlist_id));
    let subtitle = snapshot
        .playlist
        .as_ref()
        .map(|item| format!("{} 首 · {}", item.track_count, item.creator_name))
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
        .child(page::status_banner(
            snapshot.loading,
            snapshot.error.as_deref(),
            "加载中...",
            "加载失败",
        ))
        .child(
            div()
                .w_full()
                .child(if let Some(track_list) = playlist_rows {
                    track_list
                } else {
                    page::empty_card("暂无歌曲")
                }),
        )
        .into_any_element()
}
