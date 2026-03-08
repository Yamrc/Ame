use ame_netease::NeteaseClient;
use ame_netease::api::playlist::detail::PlaylistDetailRequest;
use ame_netease::api::playlist::list::PlaylistListRequest;
use ame_netease::api::track::detail::TrackDetailRequest;
use ame_netease::api::user::playlist::UserPlaylistRequest;
use anyhow::{Context as _, Result};
use serde_json::Value;
use std::collections::HashMap;

use crate::action::runtime::block_on;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPlaylistItem {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistTrackItem {
    pub id: i64,
    pub name: String,
    pub artists: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistDetailData {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub tracks: Vec<PlaylistTrackItem>,
}

const TRACK_DETAIL_BATCH_SIZE: usize = 200;

fn compact_cover_url(raw: Option<&str>, size: u32) -> Option<String> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.contains("param=") {
        return Some(raw.to_string());
    }
    let separator = if raw.contains('?') { '&' } else { '?' };
    Some(format!("{raw}{separator}param={size}y{size}"))
}

fn parse_track_item(value: &Value) -> Option<PlaylistTrackItem> {
    let id = value["id"].as_i64()?;
    let name = value["name"].as_str().unwrap_or("未知歌曲").to_string();
    let artists = value["ar"]
        .as_array()
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| artist["name"].as_str())
                .collect::<Vec<_>>()
                .join(" / ")
        })
        .filter(|artists| !artists.is_empty())
        .unwrap_or_else(|| "未知艺人".to_string());
    Some(PlaylistTrackItem { id, name, artists })
}

fn parse_playlist_track_ids(playlist: &Value) -> Vec<i64> {
    playlist["trackIds"]
        .as_array()
        .map(|track_ids| {
            track_ids
                .iter()
                .filter_map(|item| item["id"].as_i64())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

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
            let response: Value = client
                .weapi_request(TrackDetailRequest::new(chunk.to_vec()))
                .await?;
            for song in response["songs"].as_array().into_iter().flatten() {
                if let Some(track) = parse_track_item(song) {
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

pub fn fetch_user_playlists_blocking(
    user_id: i64,
    cookie: &str,
) -> Result<Vec<LibraryPlaylistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response: serde_json::Value =
        block_on(client.weapi_request(UserPlaylistRequest::new(user_id)))?;
    let playlists = response["playlist"].as_array().cloned().unwrap_or_default();

    Ok(playlists
        .into_iter()
        .filter_map(|item| {
            let id = item["id"].as_i64()?;
            let name = item["name"].as_str().unwrap_or("").to_string();
            let track_count = item["trackCount"]
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or_default();
            let creator_name = item["creator"]["nickname"]
                .as_str()
                .unwrap_or("未知用户")
                .to_string();
            let cover_url = compact_cover_url(item["coverImgUrl"].as_str(), 256);
            Some(LibraryPlaylistItem {
                id,
                name,
                track_count,
                creator_name,
                cover_url,
            })
        })
        .collect())
}

pub fn fetch_top_playlists_blocking(
    limit: u32,
    offset: u32,
    cookie: &str,
) -> Result<Vec<LibraryPlaylistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response: serde_json::Value =
        block_on(client.weapi_request(PlaylistListRequest::new(limit, offset)))?;
    let playlists = response["playlists"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    Ok(playlists
        .into_iter()
        .filter_map(|item| {
            let id = item["id"].as_i64()?;
            let name = item["name"].as_str().unwrap_or("").to_string();
            let track_count = item["trackCount"]
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or_default();
            let creator_name = item["creator"]["nickname"]
                .as_str()
                .unwrap_or("未知用户")
                .to_string();
            let cover_url = compact_cover_url(item["coverImgUrl"].as_str(), 1024);
            Some(LibraryPlaylistItem {
                id,
                name,
                track_count,
                creator_name,
                cover_url,
            })
        })
        .collect())
}

pub fn fetch_playlist_detail_blocking(
    playlist_id: i64,
    cookie: &str,
) -> Result<PlaylistDetailData> {
    let client = NeteaseClient::with_cookie(cookie);
    let response: serde_json::Value =
        block_on(client.weapi_request(PlaylistDetailRequest::new(playlist_id)))?;
    let playlist = &response["playlist"];

    let id = playlist["id"]
        .as_i64()
        .context("playlist detail missing id")?;
    let name = playlist["name"].as_str().unwrap_or("未知歌单").to_string();
    let creator_name = playlist["creator"]["nickname"]
        .as_str()
        .unwrap_or("未知用户")
        .to_string();
    let track_count = playlist["trackCount"]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or_default();
    let partial_tracks = playlist["tracks"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|track| parse_track_item(&track))
        .collect::<Vec<_>>();

    let track_ids = parse_playlist_track_ids(playlist);
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
