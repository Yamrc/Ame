use ame_netease::api::common::models::{AlbumDto, ArtistDto, PlaylistDto, TrackDto};
use ame_netease::api::search::query::{SearchArtistDto, SearchRequest, SearchResponse, SearchType};
use ame_netease::api::track::detail::TrackDetailRequest;
use anyhow::Result;
use std::collections::HashMap;

use crate::action::runtime::{block_on, netease_client};

const TRACK_DETAIL_BATCH_SIZE: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPage<T> {
    pub items: Vec<T>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSongItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchArtistItem {
    pub id: i64,
    pub name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAlbumItem {
    pub id: i64,
    pub name: String,
    pub artist_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPlaylistItem {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub cover_url: Option<String>,
}

pub fn search_songs_blocking(
    keyword: &str,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchPage<SearchSongItem>> {
    let response = execute_search(keyword, SearchType::Song, offset, limit, cookie)?;
    let mut items = response
        .result
        .songs
        .iter()
        .filter_map(parse_song_item)
        .collect::<Vec<_>>();
    backfill_song_covers(&mut items, cookie)?;
    Ok(SearchPage {
        has_more: compute_has_more(
            response.result.has_more,
            response.result.song_count,
            offset,
            items.len(),
        ),
        items,
    })
}

pub fn search_artists_blocking(
    keyword: &str,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchPage<SearchArtistItem>> {
    let response = execute_search(keyword, SearchType::Artist, offset, limit, cookie)?;
    let items = response
        .result
        .artists
        .iter()
        .filter_map(parse_artist_item)
        .collect::<Vec<_>>();
    Ok(SearchPage {
        has_more: compute_has_more(
            response.result.has_more,
            response.result.artist_count,
            offset,
            items.len(),
        ),
        items,
    })
}

pub fn search_albums_blocking(
    keyword: &str,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchPage<SearchAlbumItem>> {
    let response = execute_search(keyword, SearchType::Album, offset, limit, cookie)?;
    let items = response
        .result
        .albums
        .iter()
        .filter_map(parse_album_item)
        .collect::<Vec<_>>();
    Ok(SearchPage {
        has_more: compute_has_more(
            response.result.has_more,
            response.result.album_count,
            offset,
            items.len(),
        ),
        items,
    })
}

pub fn search_playlists_blocking(
    keyword: &str,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchPage<SearchPlaylistItem>> {
    let response = execute_search(keyword, SearchType::Playlist, offset, limit, cookie)?;
    let items = response
        .result
        .playlists
        .iter()
        .filter_map(parse_playlist_item)
        .collect::<Vec<_>>();
    Ok(SearchPage {
        has_more: compute_has_more(
            response.result.has_more,
            response.result.playlist_count,
            offset,
            items.len(),
        ),
        items,
    })
}

fn execute_search(
    keyword: &str,
    search_type: SearchType,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchResponse> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Ok(SearchResponse::default());
    }

    let client = netease_client(cookie);
    let mut request = SearchRequest::new(keyword, search_type);
    request.offset = offset;
    request.limit = limit;
    block_on(client.weapi_request(request))
}

fn compute_has_more(explicit: bool, total: u64, offset: u32, current_len: usize) -> bool {
    explicit || total > u64::from(offset) + current_len as u64
}

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

fn parse_song_item(track: &TrackDto) -> Option<SearchSongItem> {
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

fn backfill_song_covers(items: &mut [SearchSongItem], cookie: Option<&str>) -> Result<()> {
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

fn parse_artist_item(artist: &SearchArtistDto) -> Option<SearchArtistItem> {
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

fn parse_album_item(album: &AlbumDto) -> Option<SearchAlbumItem> {
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

fn parse_playlist_item(playlist: &PlaylistDto) -> Option<SearchPlaylistItem> {
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
        track_count: u32::try_from(playlist.track_count.unwrap_or_default())
            .ok()
            .unwrap_or_default(),
        cover_url: compact_cover_url(playlist.cover_img_url.as_deref(), 256),
    })
}
