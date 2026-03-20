use std::rc::Rc;

use nekowg::{AnyElement, MouseButton, div, prelude::*, px};

use crate::component::button;
use crate::component::page;

use super::{
    NavigateHandler, PlaySongHandler, PlaylistOpenHandler, SEARCH_TYPE_CARD_COLUMNS,
    SearchCollectionState, SearchPageState, SearchRouteType, render_album_card_ref,
    render_artist_card_ref, render_playlist_card_ref, render_track_row_ref,
};

pub(crate) fn render_type_page(
    route_type: SearchRouteType,
    search_state: &SearchPageState,
    current_playing_track_id: Option<i64>,
    on_play_song: PlaySongHandler,
    on_enqueue_song: super::EnqueueSongHandler,
    on_open_playlist: PlaylistOpenHandler,
    on_load_more: NavigateHandler,
) -> AnyElement {
    match route_type {
        SearchRouteType::Artists => render_collection_page(
            route_type,
            &search_state.artists,
            page::grid_or_empty(
                search_state
                    .artists
                    .items
                    .data
                    .iter()
                    .map(render_artist_card_ref)
                    .collect(),
                super::PLAYLIST_GRID_COLUMNS,
                px(24.),
                "暂无艺人结果",
            ),
            on_load_more,
        ),
        SearchRouteType::Albums => render_collection_page(
            route_type,
            &search_state.albums,
            page::grid_or_empty(
                search_state
                    .albums
                    .items
                    .data
                    .iter()
                    .map(render_album_card_ref)
                    .collect(),
                SEARCH_TYPE_CARD_COLUMNS,
                px(24.),
                "暂无专辑结果",
            ),
            on_load_more,
        ),
        SearchRouteType::Tracks => render_collection_page(
            route_type,
            &search_state.tracks,
            {
                let rows = search_state
                    .tracks
                    .items
                    .data
                    .iter()
                    .enumerate()
                    .map(|(index, song)| {
                        let is_playing = current_playing_track_id == Some(song.id);
                        let song_for_play = song.clone();
                        let song_for_enqueue = song.clone();
                        let on_play_song = on_play_song.clone();
                        let on_enqueue_song = on_enqueue_song.clone();
                        render_track_row_ref(
                            format!("search-type-track-{index}-{}", song.id),
                            song,
                            is_playing,
                            move |cx| on_play_song(song_for_play.clone(), cx),
                            move |cx| on_enqueue_song(song_for_enqueue.clone(), cx),
                        )
                    })
                    .collect::<Vec<_>>();
                if rows.is_empty() {
                    page::empty_card("暂无歌曲结果")
                } else {
                    page::stacked_rows(rows, px(8.))
                }
            },
            on_load_more,
        ),
        SearchRouteType::Playlists => render_collection_page(
            route_type,
            &search_state.playlists,
            page::grid_or_empty(
                search_state
                    .playlists
                    .items
                    .data
                    .iter()
                    .map(|playlist| {
                        let playlist_id = playlist.id;
                        let on_open_playlist = on_open_playlist.clone();
                        render_playlist_card_ref(
                            playlist,
                            Some(Rc::new(move |cx| on_open_playlist(playlist_id, cx))),
                        )
                    })
                    .collect(),
                SEARCH_TYPE_CARD_COLUMNS,
                px(24.),
                "暂无歌单结果",
            ),
            on_load_more,
        ),
    }
}

fn render_collection_page<T>(
    route_type: SearchRouteType,
    state: &SearchCollectionState<T>,
    body: AnyElement,
    on_load_more: NavigateHandler,
) -> AnyElement {
    let status = page::status_banner(
        state.items.loading,
        state.items.error.as_deref(),
        format!("正在搜索{}...", route_type.label()),
        format!("{}搜索失败", route_type.label()),
    );
    let load_more = if state.has_more && !state.items.loading && state.items.error.is_none() {
        Some(
            div()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    button::pill_base("加载更多")
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_load_more(cx)),
                )
                .into_any_element(),
        )
    } else {
        None
    };

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_5()
        .child(status)
        .child(body)
        .children(load_more)
        .into_any_element()
}
