use ame_netease::api::common::models::TrackDto;

use super::super::models::{DailyTrackItem, FmTrackItem, PlaylistTrackItem};
use super::helpers::{compact_cover_url, display_name, parse_artist_names, parse_track_alias};

pub(in crate::domain::library::service) fn parse_track_item(
    track: &TrackDto,
) -> Option<PlaylistTrackItem> {
    if track.id <= 0 {
        return None;
    }
    let alias = parse_track_alias(track);
    let artists = parse_artist_names(&track.artists);
    let album = track
        .album
        .name
        .clone()
        .filter(|value| !value.trim().is_empty());
    let cover_url = compact_cover_url(
        track.album.pic_url.as_deref().or(track.pic_url.as_deref()),
        256,
    );
    Some(PlaylistTrackItem {
        id: track.id,
        name: display_name(track.name.as_deref(), "未知歌曲"),
        alias,
        artists,
        album,
        duration_ms: track.duration_ms,
        cover_url,
    })
}

pub(in crate::domain::library::service) fn parse_fm_track_item(
    track: &TrackDto,
) -> Option<FmTrackItem> {
    if track.id <= 0 {
        return None;
    }
    let alias = parse_track_alias(track);
    let artists = parse_artist_names(&track.artists);
    let album = track
        .album
        .name
        .clone()
        .filter(|value| !value.trim().is_empty());
    let cover_url = compact_cover_url(track.album.pic_url.as_deref(), 256);
    Some(FmTrackItem {
        id: track.id,
        name: display_name(track.name.as_deref(), "未知歌曲"),
        alias,
        artists,
        album,
        duration_ms: track.duration_ms,
        cover_url,
    })
}

pub(in crate::domain::library::service) fn parse_daily_track_item(
    track: &TrackDto,
) -> Option<DailyTrackItem> {
    if track.id <= 0 {
        return None;
    }
    let alias = parse_track_alias(track);
    let artists = parse_artist_names(&track.artists);
    let album = track
        .album
        .name
        .clone()
        .filter(|value| !value.trim().is_empty());
    let cover_url = compact_cover_url(
        track.album.pic_url.as_deref().or(track.pic_url.as_deref()),
        256,
    );
    Some(DailyTrackItem {
        id: track.id,
        name: display_name(track.name.as_deref(), "未知歌曲"),
        alias,
        artists,
        album,
        duration_ms: track.duration_ms,
        cover_url,
    })
}
