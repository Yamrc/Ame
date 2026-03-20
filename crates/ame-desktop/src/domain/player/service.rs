use ame_netease::api::track::detail::TrackDetailRequest;
use ame_netease::api::track::url::TrackUrlRequest;
use anyhow::{Context as _, Result};

use crate::domain::runtime::{block_on, netease_client};

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

pub fn fetch_track_url_blocking(track_id: i64, cookie: Option<&str>) -> Result<String> {
    let client = netease_client(cookie);
    let response = block_on(client.eapi_request(TrackUrlRequest::with_level(
        vec![track_id],
        // TODO: 音质切换
        "jymaster".to_string(),
    )))?;

    response
        .data
        .first()
        .and_then(|item| item.url.as_deref())
        .filter(|url| !url.is_empty())
        .map(ToString::to_string)
        .context("track url missing in response")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMetadata {
    pub artists: String,
    pub alias: Option<String>,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

pub fn fetch_track_metadata_blocking(track_id: i64, cookie: Option<&str>) -> Result<TrackMetadata> {
    let client = netease_client(cookie);
    let response = block_on(client.eapi_request(TrackDetailRequest::new(vec![track_id])))?;

    let song = response
        .songs
        .first()
        .context("track detail missing song in response")?;

    let artists = song
        .artists
        .iter()
        .filter_map(|artist| artist.name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    let alias = song
        .tns
        .iter()
        .chain(song.alia.iter())
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");

    let cover_url = compact_cover_url(song.album.pic_url.as_deref(), 64);
    let album = song
        .album
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    Ok(TrackMetadata {
        artists: if artists.is_empty() {
            "未知艺人".to_string()
        } else {
            artists
        },
        alias: (!alias.is_empty()).then_some(alias),
        album,
        duration_ms: song.duration_ms,
        cover_url,
    })
}
