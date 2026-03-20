use ame_netease::api::common::models::{AlbumDto, ArtistDto, PlaylistDto};
use ame_netease::api::playlist::toplist::ToplistEntryDto;

use super::super::models::{AlbumItem, ArtistItem, LibraryPlaylistItem, ToplistItem};
use super::helpers::{compact_cover_url, display_name, parse_track_count_or_zero, sanitize_name};

pub(in crate::domain::library::service) fn parse_playlist_item(
    playlist: &PlaylistDto,
    cover_size: u32,
) -> Option<LibraryPlaylistItem> {
    if playlist.id <= 0 {
        return None;
    }
    let creator_name = playlist
        .creator
        .as_ref()
        .and_then(|creator| sanitize_name(creator.name.as_deref()))
        .or_else(|| sanitize_name(playlist.creator_name.as_deref()))
        .unwrap_or_else(|| "网易云音乐".to_string());
    let cover_url = compact_cover_url(playlist.cover_img_url.as_deref(), cover_size);
    Some(LibraryPlaylistItem {
        id: playlist.id,
        name: playlist.name.clone().unwrap_or_default(),
        track_count: parse_track_count_or_zero(
            playlist.track_count.unwrap_or_default(),
            "playlist.list.track_count",
            playlist.id,
        ),
        creator_name,
        cover_url,
        creator_id: playlist
            .creator
            .as_ref()
            .and_then(|creator| creator.user_id)
            .or(playlist.creator_id),
        subscribed: playlist.subscribed.unwrap_or(false),
        special_type: playlist.special_type.unwrap_or_default(),
    })
}

pub(in crate::domain::library::service) fn parse_artist_item(
    artist: &ArtistDto,
) -> Option<ArtistItem> {
    if artist.id <= 0 {
        return None;
    }
    let cover_url = compact_cover_url(artist.pic_url.as_deref(), 256);
    Some(ArtistItem {
        id: artist.id,
        name: display_name(artist.name.as_deref(), "未知艺人"),
        cover_url,
    })
}

pub(in crate::domain::library::service) fn parse_album_item(album: &AlbumDto) -> Option<AlbumItem> {
    if album.id <= 0 {
        return None;
    }
    let artist_name = album
        .artist
        .as_ref()
        .and_then(|artist| sanitize_name(artist.name.as_deref()))
        .or_else(|| {
            album
                .artists
                .first()
                .and_then(|artist| sanitize_name(artist.name.as_deref()))
        })
        .unwrap_or_else(|| "未知艺人".to_string());
    let cover_url = compact_cover_url(album.pic_url.as_deref(), 256);
    Some(AlbumItem {
        id: album.id,
        name: display_name(album.name.as_deref(), "未知专辑"),
        artist_name,
        cover_url,
    })
}

pub(in crate::domain::library::service) fn parse_toplist_item(
    playlist: &ToplistEntryDto,
) -> Option<ToplistItem> {
    if playlist.id <= 0 {
        return None;
    }
    let cover_url = compact_cover_url(playlist.cover_img_url.as_deref(), 256);
    Some(ToplistItem {
        id: playlist.id,
        name: display_name(playlist.name.as_deref(), "未知榜单"),
        update_frequency: playlist.update_frequency.clone().unwrap_or_default(),
        cover_url,
    })
}
