use std::collections::HashMap;

use ame_netease::NeteaseClient;
use ame_netease::api::playlist::detail::PlaylistDetailRequest;
use ame_netease::api::track::detail::TrackDetailRequest;
use anyhow::{Context as _, Result};

use crate::domain::runtime::block_on;

use super::models::{PlaylistDetailData, PlaylistTrackItem};
use super::parse;

const TRACK_DETAIL_BATCH_SIZE: usize = 200;

fn fetch_tracks_by_ids_blocking(track_ids: &[i64], cookie: &str) -> Result<Vec<PlaylistTrackItem>> {
    if track_ids.is_empty() {
        return Ok(Vec::new());
    }

    let cookie = cookie.to_string();
    let ids = track_ids.to_vec();
    let order_ids = ids.clone();

    let by_id = block_on(async move {
        let client = NeteaseClient::with_cookie(cookie);

        let mut by_id = HashMap::with_capacity(ids.len());

        for chunk in ids.chunks(TRACK_DETAIL_BATCH_SIZE) {
            let response = client
                .eapi_request(TrackDetailRequest::new(chunk.to_vec()))
                .await?;
            for song in &response.songs {
                if let Some(track) = parse::parse_track_item(song) {
                    by_id.entry(track.id).or_insert(track);
                }
            }
        }
        Ok::<_, ame_netease::ClientError>(by_id)
    })
    .context("failed to fetch full playlist tracks by trackIds")?;

    Ok(order_ids
        .into_iter()
        .filter_map(|id| by_id.get(&id).cloned())
        .collect::<Vec<_>>())
}

pub fn fetch_playlist_detail_blocking(
    playlist_id: i64,
    cookie: &str,
) -> Result<PlaylistDetailData> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(PlaylistDetailRequest::new(playlist_id)))?;
    let playlist = response.playlist;

    let id = playlist.id;
    anyhow::ensure!(id > 0, "playlist detail missing id");
    let name = if playlist.name.trim().is_empty() {
        "未知歌单".to_string()
    } else {
        playlist.name.clone()
    };
    let creator_name = if playlist
        .creator
        .name
        .as_deref()
        .is_none_or(|name| name.trim().is_empty())
    {
        "未知用户".to_string()
    } else {
        playlist.creator.name.clone().unwrap_or_default()
    };
    let track_count = parse::parse_track_count_or_zero(
        playlist.track_count,
        "playlist.detail.track_count",
        playlist.id,
    );
    let partial_tracks = playlist
        .tracks
        .iter()
        .filter_map(parse::parse_track_item)
        .collect::<Vec<_>>();

    let track_ids = playlist
        .track_ids
        .iter()
        .map(|track| track.id)
        .collect::<Vec<_>>();
    let tracks = if track_ids.is_empty() {
        partial_tracks
    } else {
        let mut full_tracks = fetch_tracks_by_ids_blocking(&track_ids, cookie)
            .context("failed to fetch full playlist tracks by trackIds")?;
        if full_tracks.len() == track_ids.len() {
            full_tracks
        } else {
            let mut by_id = full_tracks
                .drain(..)
                .map(|track| (track.id, track))
                .collect::<HashMap<_, _>>();
            for track in partial_tracks {
                by_id.entry(track.id).or_insert(track);
            }
            track_ids
                .into_iter()
                .filter_map(|id| by_id.remove(&id))
                .collect()
        }
    };

    Ok(PlaylistDetailData {
        id,
        name,
        creator_name,
        track_count,
        tracks,
    })
}
