use ame_netease::NeteaseClient;
use ame_netease::api::album::new::AlbumNewRequest;
use ame_netease::api::artist::toplist::ToplistArtistRequest;
use ame_netease::api::playlist::detail::PlaylistDetailRequest;
use ame_netease::api::playlist::list::PlaylistListRequest;
use ame_netease::api::playlist::personalized::PersonalizedPlaylistRequest;
use ame_netease::api::playlist::recommend_resource::RecommendResourceRequest;
use ame_netease::api::playlist::recommend_songs::RecommendSongsRequest;
use ame_netease::api::playlist::toplist::ToplistRequest;
use ame_netease::api::radio::personal_fm::PersonalFmRequest;
use ame_netease::api::track::detail::TrackDetailRequest;
use ame_netease::api::user::playlist::UserPlaylistRequest;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::action::runtime::block_on;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryPlaylistItem {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistTrackItem {
    pub id: i64,
    pub name: String,
    pub artists: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistDetailData {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub tracks: Vec<PlaylistTrackItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FmTrackItem {
    pub id: i64,
    pub name: String,
    pub artists: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyTrackItem {
    pub id: i64,
    pub name: String,
    pub artists: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistItem {
    pub id: i64,
    pub name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumItem {
    pub id: i64,
    pub name: String,
    pub artist_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToplistItem {
    pub id: i64,
    pub name: String,
    pub update_frequency: String,
    pub cover_url: Option<String>,
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

fn parse_artist_names(value: &Value) -> String {
    let artists = value["ar"]
        .as_array()
        .or_else(|| value["artists"].as_array())
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| artist["name"].as_str())
                .collect::<Vec<_>>()
                .join(" / ")
        })
        .unwrap_or_default();
    if artists.is_empty() {
        "未知艺人".to_string()
    } else {
        artists
    }
}

fn parse_track_item(value: &Value) -> Option<PlaylistTrackItem> {
    let id = value["id"].as_i64()?;
    let name = value["name"].as_str().unwrap_or("未知歌曲").to_string();
    let artists = parse_artist_names(value);
    let cover_url = compact_cover_url(
        value["al"]["picUrl"]
            .as_str()
            .or_else(|| value["album"]["picUrl"].as_str())
            .or_else(|| value["picUrl"].as_str()),
        256,
    );
    Some(PlaylistTrackItem {
        id,
        name,
        artists,
        cover_url,
    })
}

fn parse_playlist_item(value: &Value, cover_size: u32) -> Option<LibraryPlaylistItem> {
    let id = value["id"].as_i64()?;
    let name = value["name"].as_str().unwrap_or("").to_string();
    let track_count = value["trackCount"]
        .as_u64()
        .or_else(|| value["track_count"].as_u64())
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or_default();
    let creator_name = value["creator"]["nickname"]
        .as_str()
        .or_else(|| value["creatorName"].as_str())
        .or_else(|| value["creator"]["name"].as_str())
        .unwrap_or("网易云音乐")
        .to_string();
    let cover_url = compact_cover_url(
        value["coverImgUrl"]
            .as_str()
            .or_else(|| value["picUrl"].as_str()),
        cover_size,
    );
    Some(LibraryPlaylistItem {
        id,
        name,
        track_count,
        creator_name,
        cover_url,
    })
}

fn parse_fm_track_item(value: &Value) -> Option<FmTrackItem> {
    let id = value["id"].as_i64()?;
    let name = value["name"].as_str().unwrap_or("未知歌曲").to_string();
    let artists = parse_artist_names(value);
    let cover_url = compact_cover_url(
        value["album"]["picUrl"]
            .as_str()
            .or_else(|| value["al"]["picUrl"].as_str()),
        256,
    );
    Some(FmTrackItem {
        id,
        name,
        artists,
        cover_url,
    })
}

fn parse_daily_track_item(value: &Value) -> Option<DailyTrackItem> {
    let id = value["id"].as_i64()?;
    let name = value["name"].as_str().unwrap_or("未知歌曲").to_string();
    let artists = parse_artist_names(value);
    let cover_url = compact_cover_url(
        value["al"]["picUrl"]
            .as_str()
            .or_else(|| value["album"]["picUrl"].as_str())
            .or_else(|| value["picUrl"].as_str()),
        256,
    );
    Some(DailyTrackItem {
        id,
        name,
        artists,
        cover_url,
    })
}

fn parse_artist_item(value: &Value) -> Option<ArtistItem> {
    let id = value["id"].as_i64()?;
    let name = value["name"].as_str().unwrap_or("未知艺人").to_string();
    let cover_url = compact_cover_url(value["picUrl"].as_str(), 256);
    Some(ArtistItem {
        id,
        name,
        cover_url,
    })
}

fn parse_album_item(value: &Value) -> Option<AlbumItem> {
    let id = value["id"].as_i64()?;
    let name = value["name"].as_str().unwrap_or("未知专辑").to_string();
    let artist_name = value["artist"]["name"]
        .as_str()
        .or_else(|| {
            value["artists"]
                .as_array()
                .and_then(|artists| artists.first())
                .and_then(|artist| artist["name"].as_str())
        })
        .unwrap_or("未知艺人")
        .to_string();
    let cover_url = compact_cover_url(
        value["picUrl"]
            .as_str()
            .or_else(|| value["coverImgUrl"].as_str()),
        256,
    );
    Some(AlbumItem {
        id,
        name,
        artist_name,
        cover_url,
    })
}

fn parse_toplist_item(value: &Value) -> Option<ToplistItem> {
    let id = value["id"].as_i64()?;
    let name = value["name"].as_str().unwrap_or("未知榜单").to_string();
    let update_frequency = value["updateFrequency"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let cover_url = compact_cover_url(value["coverImgUrl"].as_str(), 256);
    Some(ToplistItem {
        id,
        name,
        update_frequency,
        cover_url,
    })
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
        .filter_map(|item| parse_playlist_item(&item, 256))
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
        .filter_map(|item| parse_playlist_item(&item, 1024))
        .collect())
}

pub fn fetch_personalized_playlists_blocking(
    limit: u32,
    cookie: &str,
) -> Result<Vec<LibraryPlaylistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response: serde_json::Value =
        block_on(client.weapi_request(PersonalizedPlaylistRequest::new(limit)))?;
    let playlists = response["result"].as_array().cloned().unwrap_or_default();
    Ok(playlists
        .into_iter()
        .filter_map(|item| parse_playlist_item(&item, 256))
        .collect())
}

pub fn fetch_daily_recommend_playlists_blocking(cookie: &str) -> Result<Vec<LibraryPlaylistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response: serde_json::Value =
        block_on(client.weapi_request(RecommendResourceRequest::new()))?;
    let playlists = response["recommend"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    Ok(playlists
        .into_iter()
        .filter_map(|item| parse_playlist_item(&item, 256))
        .collect())
}

pub fn fetch_daily_recommend_tracks_blocking(cookie: &str) -> Result<Vec<DailyTrackItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response: serde_json::Value = block_on(client.weapi_request(RecommendSongsRequest::new()))?;
    let tracks = response["data"]["dailySongs"]
        .as_array()
        .or_else(|| response["dailySongs"].as_array())
        .cloned()
        .unwrap_or_default();
    Ok(tracks
        .into_iter()
        .filter_map(|track| parse_daily_track_item(&track))
        .collect())
}

pub fn fetch_personal_fm_blocking(cookie: &str) -> Result<Option<FmTrackItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response: serde_json::Value = block_on(client.weapi_request(PersonalFmRequest::new()))?;
    let tracks = response["data"].as_array().cloned().unwrap_or_default();
    Ok(tracks
        .into_iter()
        .find_map(|track| parse_fm_track_item(&track)))
}

pub fn fetch_recommend_artists_blocking(
    artist_type: u32,
    limit: u32,
    cookie: &str,
) -> Result<Vec<ArtistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let pool = limit.max(60);
    let response: serde_json::Value =
        block_on(client.weapi_request(ToplistArtistRequest::new(artist_type, pool, 0)))?;
    let artists = response["list"]["artists"]
        .as_array()
        .or_else(|| response["artists"].as_array())
        .cloned()
        .unwrap_or_default();
    let mut items = artists
        .into_iter()
        .filter_map(|artist| parse_artist_item(&artist))
        .collect::<Vec<_>>();

    let target = limit as usize;
    if items.len() < target {
        let offset = items.len() as u32;
        let response: serde_json::Value =
            block_on(client.weapi_request(ToplistArtistRequest::new(artist_type, pool, offset)))?;
        let more = response["list"]["artists"]
            .as_array()
            .or_else(|| response["artists"].as_array())
            .cloned()
            .unwrap_or_default();
        for artist in more
            .into_iter()
            .filter_map(|artist| parse_artist_item(&artist))
        {
            if items.iter().any(|existing| existing.id == artist.id) {
                continue;
            }
            items.push(artist);
            if items.len() >= target {
                break;
            }
        }
    }

    if items.len() > 1 {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        for i in (1..items.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let j = (seed as usize) % (i + 1);
            items.swap(i, j);
        }
    }

    if items.len() > target {
        items.truncate(target);
    }

    Ok(items)
}

pub fn fetch_new_albums_blocking(
    limit: u32,
    offset: u32,
    area: &str,
    cookie: &str,
) -> Result<Vec<AlbumItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response: serde_json::Value =
        block_on(client.weapi_request(AlbumNewRequest::new(limit, offset, area)))?;
    let albums = response["albums"].as_array().cloned().unwrap_or_default();
    Ok(albums
        .into_iter()
        .filter_map(|album| parse_album_item(&album))
        .collect())
}

pub fn fetch_toplists_blocking(cookie: &str) -> Result<Vec<ToplistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response: serde_json::Value = block_on(client.eapi_request(ToplistRequest::new()))?;
    let lists = response["list"].as_array().cloned().unwrap_or_default();
    Ok(lists
        .into_iter()
        .filter_map(|item| parse_toplist_item(&item))
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
