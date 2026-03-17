use ame_netease::NeteaseClient;
use ame_netease::api::album::new::AlbumNewRequest;
use ame_netease::api::artist::toplist::ToplistArtistRequest;
use ame_netease::api::common::models::{AlbumDto, ArtistDto, PlaylistDto, TrackDto};
use ame_netease::api::playlist::detail::PlaylistDetailRequest;
use ame_netease::api::playlist::list::PlaylistListRequest;
use ame_netease::api::playlist::personalized::PersonalizedPlaylistRequest;
use ame_netease::api::playlist::recommend_resource::RecommendResourceRequest;
use ame_netease::api::playlist::recommend_songs::RecommendSongsRequest;
use ame_netease::api::playlist::toplist::{ToplistEntryDto, ToplistRequest};
use ame_netease::api::radio::personal_fm::PersonalFmRequest;
use ame_netease::api::track::detail::TrackDetailRequest;
use ame_netease::api::track::lyric::TrackLyricRequest;
use ame_netease::api::user::playlist::UserPlaylistRequest;
use anyhow::{Context as _, Result};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::action::runtime::block_on;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryPlaylistItem {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
    #[serde(default)]
    pub creator_id: Option<i64>,
    #[serde(default)]
    pub subscribed: bool,
    #[serde(default)]
    pub special_type: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistTrackItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
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
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyTrackItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
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

fn parse_artist_names(artists: &[ArtistDto]) -> String {
    let artists = artists
        .iter()
        .filter_map(|artist| artist.name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    if artists.is_empty() {
        "未知艺人".to_string()
    } else {
        artists
    }
}

fn parse_track_alias(track: &TrackDto) -> Option<String> {
    let alias = track
        .tns
        .iter()
        .chain(track.trans_names.iter())
        .chain(track.alia.iter())
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    (!alias.is_empty()).then_some(alias)
}

fn parse_track_item(track: &TrackDto) -> Option<PlaylistTrackItem> {
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
        name: if track
            .name
            .as_deref()
            .is_none_or(|name| name.trim().is_empty())
        {
            "未知歌曲".to_string()
        } else {
            track.name.clone().unwrap_or_default()
        },
        alias,
        artists,
        album,
        duration_ms: track.duration_ms,
        cover_url,
    })
}

fn parse_playlist_item(playlist: &PlaylistDto, cover_size: u32) -> Option<LibraryPlaylistItem> {
    if playlist.id <= 0 {
        return None;
    }
    let creator_name = playlist
        .creator
        .as_ref()
        .and_then(|creator| creator.name.as_deref())
        .map(str::trim)
        .map(ToString::to_string)
        .and_then(|name| {
            name.split_once('\0')
                .map(|(name, _)| name.to_string())
                .or(Some(name))
        })
        .filter(|name| !name.is_empty())
        .or_else(|| {
            playlist
                .creator_name
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "网易云音乐".to_string());
    let cover_url = compact_cover_url(playlist.cover_img_url.as_deref(), cover_size);
    Some(LibraryPlaylistItem {
        id: playlist.id,
        name: playlist.name.clone().unwrap_or_default(),
        track_count: u32::try_from(playlist.track_count.unwrap_or_default())
            .ok()
            .unwrap_or_default(),
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

fn parse_fm_track_item(track: &TrackDto) -> Option<FmTrackItem> {
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
        name: if track
            .name
            .as_deref()
            .is_none_or(|name| name.trim().is_empty())
        {
            "未知歌曲".to_string()
        } else {
            track.name.clone().unwrap_or_default()
        },
        alias,
        artists,
        album,
        duration_ms: track.duration_ms,
        cover_url,
    })
}

fn parse_daily_track_item(track: &TrackDto) -> Option<DailyTrackItem> {
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
        name: if track
            .name
            .as_deref()
            .is_none_or(|name| name.trim().is_empty())
        {
            "未知歌曲".to_string()
        } else {
            track.name.clone().unwrap_or_default()
        },
        alias,
        artists,
        album,
        duration_ms: track.duration_ms,
        cover_url,
    })
}

fn parse_artist_item(artist: &ArtistDto) -> Option<ArtistItem> {
    if artist.id <= 0 {
        return None;
    }
    let cover_url = compact_cover_url(artist.pic_url.as_deref(), 256);
    Some(ArtistItem {
        id: artist.id,
        name: if artist
            .name
            .as_deref()
            .is_none_or(|name| name.trim().is_empty())
        {
            "未知艺人".to_string()
        } else {
            artist.name.clone().unwrap_or_default()
        },
        cover_url,
    })
}

fn parse_album_item(album: &AlbumDto) -> Option<AlbumItem> {
    if album.id <= 0 {
        return None;
    }
    let artist_name = album
        .artist
        .as_ref()
        .and_then(|artist| artist.name.as_deref())
        .map(str::trim)
        .map(ToString::to_string)
        .and_then(|name| {
            name.split_once('\0')
                .map(|(name, _)| name.to_string())
                .or(Some(name))
        })
        .filter(|name| !name.is_empty())
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
    let cover_url = compact_cover_url(album.pic_url.as_deref(), 256);
    Some(AlbumItem {
        id: album.id,
        name: if album
            .name
            .as_deref()
            .is_none_or(|name| name.trim().is_empty())
        {
            "未知专辑".to_string()
        } else {
            album.name.clone().unwrap_or_default()
        },
        artist_name,
        cover_url,
    })
}

fn parse_toplist_item(playlist: &ToplistEntryDto) -> Option<ToplistItem> {
    if playlist.id <= 0 {
        return None;
    }
    let cover_url = compact_cover_url(playlist.cover_img_url.as_deref(), 256);
    Some(ToplistItem {
        id: playlist.id,
        name: if playlist
            .name
            .as_deref()
            .is_none_or(|name| name.trim().is_empty())
        {
            "未知榜单".to_string()
        } else {
            playlist.name.clone().unwrap_or_default()
        },
        update_frequency: playlist.update_frequency.clone().unwrap_or_default(),
        cover_url,
    })
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
            let response = client
                .eapi_request(TrackDetailRequest::new(chunk.to_vec()))
                .await?;
            for song in &response.songs {
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
    let response = block_on(client.weapi_request(UserPlaylistRequest::new(user_id)))?;

    Ok(response
        .playlists
        .iter()
        .filter_map(|item| parse_playlist_item(item, 256))
        .collect())
}

pub fn fetch_top_playlists_blocking(
    limit: u32,
    offset: u32,
    cookie: &str,
) -> Result<Vec<LibraryPlaylistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(PlaylistListRequest::new(limit, offset)))?;

    Ok(response
        .playlists
        .iter()
        .filter_map(|item| parse_playlist_item(item, 1024))
        .collect())
}

pub fn fetch_personalized_playlists_blocking(
    limit: u32,
    cookie: &str,
) -> Result<Vec<LibraryPlaylistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(PersonalizedPlaylistRequest::new(limit)))?;
    Ok(response
        .result
        .iter()
        .filter_map(|item| parse_playlist_item(item, 256))
        .collect())
}

pub fn fetch_daily_recommend_playlists_blocking(cookie: &str) -> Result<Vec<LibraryPlaylistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(RecommendResourceRequest::new()))?;
    Ok(response
        .playlists
        .iter()
        .filter_map(|item| parse_playlist_item(item, 256))
        .collect())
}

pub fn fetch_daily_recommend_tracks_blocking(cookie: &str) -> Result<Vec<DailyTrackItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(RecommendSongsRequest::new()))?;
    let tracks = if response.data.daily_songs.is_empty() {
        &response.daily_songs
    } else {
        &response.data.daily_songs
    };
    Ok(tracks.iter().filter_map(parse_daily_track_item).collect())
}

pub fn fetch_personal_fm_blocking(cookie: &str) -> Result<Option<FmTrackItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(PersonalFmRequest::new()))?;
    Ok(response.data.iter().find_map(parse_fm_track_item))
}

pub fn fetch_recommend_artists_blocking(
    artist_type: u32,
    limit: u32,
    cookie: &str,
) -> Result<Vec<ArtistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let pool = limit.max(60);
    let response = block_on(client.weapi_request(ToplistArtistRequest::new(artist_type, pool, 0)))?;
    let mut items = response
        .list
        .artists
        .iter()
        .chain(response.artists.iter())
        .filter_map(parse_artist_item)
        .collect::<Vec<_>>();

    let target = limit as usize;
    if items.len() < target {
        let offset = items.len() as u32;
        let response =
            block_on(client.weapi_request(ToplistArtistRequest::new(artist_type, pool, offset)))?;
        for artist in response
            .list
            .artists
            .iter()
            .chain(response.artists.iter())
            .filter_map(parse_artist_item)
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
    let response = block_on(client.weapi_request(AlbumNewRequest::new(limit, offset, area)))?;
    Ok(response
        .albums
        .iter()
        .filter_map(parse_album_item)
        .collect())
}

pub fn fetch_toplists_blocking(cookie: &str) -> Result<Vec<ToplistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.eapi_request(ToplistRequest::new()))?;
    Ok(response
        .list
        .iter()
        .filter_map(parse_toplist_item)
        .collect())
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
    let track_count = u32::try_from(playlist.track_count).ok().unwrap_or_default();
    let partial_tracks = playlist
        .tracks
        .iter()
        .filter_map(parse_track_item)
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

pub fn fetch_track_lyric_preview_blocking(track_id: i64, cookie: &str) -> Result<Vec<String>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(TrackLyricRequest::new(track_id)))?;
    let raw = response.main_lyric().unwrap_or_default();
    let mut lines = parse_lyric_lines(raw);
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let pick = 2.min(lines.len());
    let mut rng = rand::rng();
    let start = if lines.len() <= pick {
        0
    } else {
        rng.random_range(0..=lines.len() - pick)
    };
    lines = lines.into_iter().skip(start).take(pick).collect();
    Ok(lines)
}

fn parse_lyric_lines(raw: &str) -> Vec<String> {
    raw.lines()
        .filter_map(|line| line.split(']').next_back())
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .filter(|line| {
            !line.contains("作词")
                && !line.contains("作曲")
                && !line.contains("纯音乐")
                && !line.contains("编曲")
        })
        .map(|line| line.to_string())
        .collect()
}
