use ame_netease::api::track::detail::TrackDetailRequest;
use ame_netease::api::track::url::TrackUrlRequest;
use anyhow::{Context as _, Result};

use crate::action::runtime::{block_on, netease_client};

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
    let response: serde_json::Value = block_on(client.eapi_request(TrackUrlRequest::with_level(
        vec![track_id],
        "jymaster".to_string(),
    )))?;

    response["data"]
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item["url"].as_str())
        .filter(|url| !url.is_empty())
        .map(ToString::to_string)
        .context("track url missing in response")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMetadata {
    pub artists: String,
    pub alias: Option<String>,
    pub cover_url: Option<String>,
}

pub fn fetch_track_metadata_blocking(track_id: i64, cookie: Option<&str>) -> Result<TrackMetadata> {
    let client = netease_client(cookie);
    let response: serde_json::Value =
        block_on(client.weapi_request(TrackDetailRequest::new(vec![track_id])))?;

    let song = response["songs"]
        .as_array()
        .and_then(|songs| songs.first())
        .context("track detail missing song in response")?;

    let artists = song["ar"]
        .as_array()
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| artist["name"].as_str().map(ToString::to_string))
                .collect::<Vec<String>>()
                .join(" / ")
        })
        .filter(|artists| !artists.is_empty())
        .unwrap_or_else(|| "未知艺人".to_string());
    let alias = song["tns"]
        .as_array()
        .or_else(|| song["alia"].as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::trim))
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
                .join(" / ")
        })
        .filter(|alias| !alias.is_empty());

    let cover_url = compact_cover_url(song["al"]["picUrl"].as_str(), 64);

    Ok(TrackMetadata {
        artists,
        alias,
        cover_url,
    })
}
