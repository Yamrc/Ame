use ame_netease::api::common::models::{AlbumDto, PlaylistDto};
use ame_netease::api::search::query::SearchArtistDto;

use super::super::models::{SearchAlbumItem, SearchArtistItem, SearchPlaylistItem};
use super::helpers::{compact_cover_url, parse_track_count_or_zero};

pub(in crate::domain::search::service) fn parse_artist_item(
    artist: &SearchArtistDto,
) -> Option<SearchArtistItem> {
    if artist.id <= 0 {
        return None;
    }
    Some(SearchArtistItem {
        id: artist.id,
        name: artist
            .name
            .clone()
            .unwrap_or_else(|| "未知艺人".to_string()),
        cover_url: compact_cover_url(
            artist.pic_url.as_deref().or(artist.img1v1_url.as_deref()),
            256,
        ),
    })
}

pub(in crate::domain::search::service) fn parse_album_item(
    album: &AlbumDto,
) -> Option<SearchAlbumItem> {
    if album.id <= 0 {
        return None;
    }
    let artist_name = album
        .artist
        .as_ref()
        .and_then(|artist| artist.name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            album
                .artists
                .first()
                .and_then(|artist| artist.name.as_deref())
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "未知艺人".to_string());
    Some(SearchAlbumItem {
        id: album.id,
        name: album.name.clone().unwrap_or_else(|| "未知专辑".to_string()),
        artist_name,
        cover_url: compact_cover_url(album.pic_url.as_deref(), 256),
    })
}

pub(in crate::domain::search::service) fn parse_playlist_item(
    playlist: &PlaylistDto,
) -> Option<SearchPlaylistItem> {
    if playlist.id <= 0 {
        return None;
    }
    let creator_name = playlist
        .creator
        .as_ref()
        .and_then(|creator| creator.name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            playlist
                .creator_name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "网易云音乐".to_string());
    Some(SearchPlaylistItem {
        id: playlist.id,
        name: playlist
            .name
            .clone()
            .unwrap_or_else(|| "未知歌单".to_string()),
        creator_name,
        track_count: parse_track_count_or_zero(
            playlist.track_count.unwrap_or_default(),
            playlist.id,
        ),
        cover_url: compact_cover_url(playlist.cover_img_url.as_deref(), 256),
    })
}
