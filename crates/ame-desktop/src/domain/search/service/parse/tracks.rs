use ame_netease::api::common::models::TrackDto;
use ame_netease::api::track::detail::TrackDetailRequest;
use anyhow::Result;
use std::collections::HashMap;

use crate::domain::runtime::{block_on, netease_client};

use super::super::models::SearchSongItem;
use super::helpers::{
    TRACK_DETAIL_BATCH_SIZE, compact_cover_url, parse_artist_names, parse_track_alias,
};

pub(in crate::domain::search::service) fn parse_song_item(
    track: &TrackDto,
) -> Option<SearchSongItem> {
    if track.id <= 0 {
        return None;
    }
    let album = track
        .album
        .name
        .clone()
        .filter(|value| !value.trim().is_empty());
    Some(SearchSongItem {
        id: track.id,
        name: track.name.clone().unwrap_or_else(|| "未知歌曲".to_string()),
        alias: parse_track_alias(track),
        artists: parse_artist_names(&track.artists),
        album,
        duration_ms: track.duration_ms,
        cover_url: compact_cover_url(
            track.album.pic_url.as_deref().or(track.pic_url.as_deref()),
            256,
        ),
    })
}

pub(in crate::domain::search::service) fn backfill_song_covers(
    items: &mut [SearchSongItem],
    cookie: Option<&str>,
) -> Result<()> {
    let missing_cover_ids = items
        .iter()
        .filter(|item| item.cover_url.is_none())
        .map(|item| item.id)
        .collect::<Vec<_>>();
    if missing_cover_ids.is_empty() {
        return Ok(());
    }

    let client = netease_client(cookie);
    let mut cover_by_id = HashMap::new();
    for chunk in missing_cover_ids.chunks(TRACK_DETAIL_BATCH_SIZE) {
        let response = block_on(client.eapi_request(TrackDetailRequest::new(chunk.to_vec())))?;
        for song in response.songs {
            let cover_url = compact_cover_url(
                song.album.pic_url.as_deref().or(song.pic_url.as_deref()),
                256,
            );
            if let Some(cover_url) = cover_url {
                cover_by_id.insert(song.id, cover_url);
            }
        }
    }

    for item in items.iter_mut() {
        if item.cover_url.is_none() {
            item.cover_url = cover_by_id.get(&item.id).cloned();
        }
    }
    Ok(())
}
