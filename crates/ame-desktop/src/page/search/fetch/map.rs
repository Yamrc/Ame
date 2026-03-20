use crate::domain::search;

use super::super::types::{
    SearchAlbum, SearchArtist, SearchOverview, SearchPageSlice, SearchPlaylist, SearchSong,
    SearchTypePayload,
};

pub(super) fn empty_payload(route_type: super::super::types::SearchRouteType) -> SearchTypePayload {
    match route_type {
        super::super::types::SearchRouteType::Artists => {
            SearchTypePayload::Artists(SearchPageSlice {
                items: Vec::new(),
                has_more: false,
            })
        }
        super::super::types::SearchRouteType::Albums => {
            SearchTypePayload::Albums(SearchPageSlice {
                items: Vec::new(),
                has_more: false,
            })
        }
        super::super::types::SearchRouteType::Tracks => {
            SearchTypePayload::Tracks(SearchPageSlice {
                items: Vec::new(),
                has_more: false,
            })
        }
        super::super::types::SearchRouteType::Playlists => {
            SearchTypePayload::Playlists(SearchPageSlice {
                items: Vec::new(),
                has_more: false,
            })
        }
    }
}

pub(super) fn map_search_overview(
    artists: search::SearchPage<search::SearchArtistItem>,
    albums: search::SearchPage<search::SearchAlbumItem>,
    tracks: search::SearchPage<search::SearchSongItem>,
    playlists: search::SearchPage<search::SearchPlaylistItem>,
) -> SearchOverview {
    SearchOverview {
        artists: artists.items.into_iter().map(map_search_artist).collect(),
        albums: albums.items.into_iter().map(map_search_album).collect(),
        tracks: tracks.items.into_iter().map(map_search_song).collect(),
        playlists: playlists
            .items
            .into_iter()
            .map(map_search_playlist)
            .collect(),
    }
}

pub(super) fn map_search_song(item: search::SearchSongItem) -> SearchSong {
    SearchSong {
        id: item.id,
        name: item.name,
        alias: item.alias,
        artists: item.artists,
        album: item.album,
        duration_ms: item.duration_ms,
        cover_url: item.cover_url,
    }
}

pub(super) fn map_search_artist(item: search::SearchArtistItem) -> SearchArtist {
    SearchArtist {
        id: item.id,
        name: item.name,
        cover_url: item.cover_url,
    }
}

pub(super) fn map_search_album(item: search::SearchAlbumItem) -> SearchAlbum {
    SearchAlbum {
        id: item.id,
        name: item.name,
        artist_name: item.artist_name,
        cover_url: item.cover_url,
    }
}

pub(super) fn map_search_playlist(item: search::SearchPlaylistItem) -> SearchPlaylist {
    SearchPlaylist {
        id: item.id,
        name: item.name,
        creator_name: if item.track_count == 0 {
            item.creator_name
        } else {
            format!("{} 首 · {}", item.track_count, item.creator_name)
        },
        track_count: item.track_count,
        cover_url: item.cover_url,
    }
}
