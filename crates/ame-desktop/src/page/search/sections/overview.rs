use std::rc::Rc;

use nekowg::{AnyElement, div, prelude::*, px};

use crate::component::{page, section};

use super::{
    EnqueueSongHandler, OVERVIEW_ARTIST_PLACEHOLDER_HEIGHT, OVERVIEW_CARD_PLACEHOLDER_HEIGHT,
    OVERVIEW_TRACK_PLACEHOLDER_HEIGHT, PLAYLIST_GRID_COLUMNS, PlaySongHandler, PlaylistOpenHandler,
    SHORT_TRACK_COLUMNS, SHORT_TRACK_GRID_GAP, SearchOverview, SearchRouteType,
    SearchTypeNavigateHandler, render_artist_card, render_playlist_card, render_short_track_item,
};

pub(crate) fn render_overview_sections(
    overview: SearchOverview,
    on_play_song: PlaySongHandler,
    on_enqueue_song: EnqueueSongHandler,
    on_open_playlist: PlaylistOpenHandler,
    on_navigate_type: SearchTypeNavigateHandler,
) -> AnyElement {
    let on_navigate_type_for_artists = on_navigate_type.clone();
    let artists = section::titled(
        "艺人",
        Some(Rc::new(move |cx| {
            on_navigate_type_for_artists(SearchRouteType::Artists, cx)
        })),
        page::grid_or_placeholder(
            overview
                .artists
                .iter()
                .take(3)
                .cloned()
                .map(|artist| render_artist_card(artist.name, artist.cover_url))
                .collect(),
            3,
            px(24.),
            "暂无艺人结果",
            px(OVERVIEW_ARTIST_PLACEHOLDER_HEIGHT),
        ),
    );
    let on_navigate_type_for_albums = on_navigate_type.clone();
    let albums = section::titled(
        "专辑",
        Some(Rc::new(move |cx| {
            on_navigate_type_for_albums(SearchRouteType::Albums, cx)
        })),
        page::grid_or_placeholder(
            overview
                .albums
                .iter()
                .take(3)
                .cloned()
                .map(|album| {
                    render_playlist_card(album.name, album.artist_name, album.cover_url, None)
                })
                .collect(),
            3,
            px(24.),
            "暂无专辑结果",
            px(OVERVIEW_CARD_PLACEHOLDER_HEIGHT),
        ),
    );
    let track_rows = overview
        .tracks
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, song)| {
            let song_for_play = song.clone();
            let song_for_enqueue = song.clone();
            let on_play_song = on_play_song.clone();
            let on_enqueue_song = on_enqueue_song.clone();
            render_short_track_item(
                format!("search-overview-track-{index}-{}", song.id),
                song,
                move |cx| on_play_song(song_for_play.clone(), cx),
                move |cx| on_enqueue_song(song_for_enqueue.clone(), cx),
            )
        })
        .collect::<Vec<_>>();
    let playlist_cards = overview
        .playlists
        .iter()
        .take(12)
        .cloned()
        .map(|playlist| {
            let playlist_id = playlist.id;
            let on_open_playlist = on_open_playlist.clone();
            render_playlist_card(
                playlist.name,
                playlist.creator_name,
                playlist.cover_url,
                Some(Rc::new(move |cx| on_open_playlist(playlist_id, cx))),
            )
        })
        .collect::<Vec<_>>();

    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_10()
        .child(
            div()
                .w_full()
                .flex()
                .items_start()
                .gap(px(48.))
                .child(div().flex_1().min_w(px(0.)).child(artists))
                .child(div().flex_1().min_w(px(0.)).child(albums)),
        )
        .child({
            let on_navigate_type = on_navigate_type.clone();
            section::titled(
                "歌曲",
                Some(Rc::new(move |cx| {
                    on_navigate_type(SearchRouteType::Tracks, cx)
                })),
                page::grid_or_placeholder(
                    track_rows,
                    SHORT_TRACK_COLUMNS,
                    px(SHORT_TRACK_GRID_GAP),
                    "暂无歌曲结果",
                    px(OVERVIEW_TRACK_PLACEHOLDER_HEIGHT),
                ),
            )
        })
        .child({
            let on_navigate_type = on_navigate_type.clone();
            section::titled(
                "歌单",
                Some(Rc::new(move |cx| {
                    on_navigate_type(SearchRouteType::Playlists, cx)
                })),
                page::grid_or_placeholder(
                    playlist_cards,
                    PLAYLIST_GRID_COLUMNS,
                    px(24.),
                    "暂无歌单结果",
                    px(OVERVIEW_CARD_PLACEHOLDER_HEIGHT),
                ),
            )
        })
        .into_any_element()
}
