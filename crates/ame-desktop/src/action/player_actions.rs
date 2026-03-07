use ame_netease::NeteaseClient;
use ame_netease::api::track::detail::TrackDetailRequest;
use ame_netease::api::track::url::TrackUrlRequest;
use anyhow::{Context as _, Result};
use std::future::Future;

fn block_on<F, T, E>(future: F) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build temporary tokio runtime")?;
    Ok(runtime.block_on(future)?)
}

pub fn fetch_track_url_blocking(track_id: i64, cookie: Option<&str>) -> Result<String> {
    let client = cookie
        .filter(|cookie| !cookie.trim().is_empty())
        .map_or_else(NeteaseClient::new, NeteaseClient::with_cookie);
    let response: serde_json::Value =
        block_on(client.eapi_request(TrackUrlRequest::with_level(vec![track_id], "jymaster".to_string())))?;

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
    pub cover_url: Option<String>,
}

pub fn fetch_track_metadata_blocking(track_id: i64, cookie: Option<&str>) -> Result<TrackMetadata> {
    let client = cookie
        .filter(|cookie| !cookie.trim().is_empty())
        .map_or_else(NeteaseClient::new, NeteaseClient::with_cookie);
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

    let cover_url = song["al"]["picUrl"]
        .as_str()
        .filter(|url| !url.is_empty())
        .map(ToString::to_string);

    Ok(TrackMetadata { artists, cover_url })
}
