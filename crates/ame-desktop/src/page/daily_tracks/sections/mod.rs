use std::sync::Arc;

use nekowg::{AnyElement, App, FontWeight, div, prelude::*, px, rgb};

use crate::component::track_item::TrackItemFavoriteState;
use crate::component::{page, theme};
use crate::domain::favorites::FavoritesState;
use crate::domain::library::DailyTrackItem;
use crate::page::playlist::{self, PlaylistTrackRow};
use crate::page::state::DataState;

pub(crate) type TrackActionHandler = Arc<dyn Fn(PlaylistTrackRow, &mut App)>;
pub(crate) type FavoriteTrackHandler = Arc<dyn Fn(i64, &mut App)>;
pub(crate) type ReplaceDailyQueueHandler = Arc<dyn Fn(Option<i64>, &mut App)>;

#[derive(Clone)]
pub(crate) struct DailyTracksFavoriteState {
    pub favorites: FavoritesState,
    pub ready: bool,
}

#[derive(Clone)]
pub(crate) struct DailyTracksRenderActions {
    pub on_play_track: TrackActionHandler,
    pub on_enqueue_track: TrackActionHandler,
    pub on_toggle_favorite: FavoriteTrackHandler,
    pub on_replace_queue: ReplaceDailyQueueHandler,
}

pub(crate) struct DailyTracksRenderCache {
    pub rows: Arc<Vec<PlaylistTrackRow>>,
    pub first_track_id: Option<i64>,
    pub current_playing_track_id: Option<i64>,
}

pub(crate) fn render_daily_tracks_page(
    state: &DataState<Vec<DailyTrackItem>>,
    render_cache: Option<&DailyTracksRenderCache>,
    favorite_state: DailyTracksFavoriteState,
    actions: DailyTracksRenderActions,
) -> AnyElement {
    let rows = render_cache
        .map(|cache| {
            let favorite_state = favorite_state.clone();
            cache
                .rows
                .iter()
                .cloned()
                .enumerate()
                .map(|(index, track)| {
                    let is_playing = cache.current_playing_track_id == Some(track.id);
                    let favorite = TrackItemFavoriteState {
                        liked: favorite_state.favorites.is_liked(track.id),
                        enabled: favorite_state.ready,
                        pending: favorite_state.favorites.is_pending(track.id),
                    };
                    let play_track = track.clone();
                    let queue_track = track.clone();
                    let toggle_track_id = track.id;
                    let on_play_track = actions.on_play_track.clone();
                    let on_enqueue_track = actions.on_enqueue_track.clone();
                    let on_toggle_favorite = actions.on_toggle_favorite.clone();
                    playlist::track_row(
                        format!("daily-tracks:row:{index}:track:{}", track.id),
                        track,
                        is_playing,
                        favorite,
                        move |cx| on_play_track(play_track.clone(), cx),
                        move |cx| on_enqueue_track(queue_track.clone(), cx),
                        move |cx| on_toggle_favorite(toggle_track_id, cx),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let action = render_cache
        .and_then(|cache| cache.first_track_id)
        .map(|track_id| {
            let on_replace_queue = actions.on_replace_queue.clone();
            crate::component::button::primary_pill("替换队列并播放")
                .on_mouse_down(nekowg::MouseButton::Left, move |_, _, cx| {
                    on_replace_queue(Some(track_id), cx);
                })
                .into_any_element()
        });
    let status = page::status_banner(
        state.loading,
        state.error.as_deref(),
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
