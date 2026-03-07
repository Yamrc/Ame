use ame_netease::NeteaseClient;
use ame_netease::api::playlist::detail::PlaylistDetailRequest;
use ame_netease::api::user::playlist::UserPlaylistRequest;
use anyhow::{Context as _, Result};
use std::future::Future;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPlaylistItem {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
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

pub fn fetch_user_playlists_blocking(
    user_id: i64,
    cookie: Option<&str>,
) -> Result<Vec<LibraryPlaylistItem>> {
    let client = cookie
        .filter(|cookie| !cookie.trim().is_empty())
        .map_or_else(NeteaseClient::new, NeteaseClient::with_cookie);
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
            Some(LibraryPlaylistItem {
                id,
                name,
                track_count,
                creator_name,
            })
        })
        .collect())
}

pub fn fetch_playlist_detail_blocking(
    playlist_id: i64,
    cookie: Option<&str>,
) -> Result<PlaylistDetailData> {
    let client = cookie
        .filter(|cookie| !cookie.trim().is_empty())
        .map_or_else(NeteaseClient::new, NeteaseClient::with_cookie);
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
    let tracks = playlist["tracks"].as_array().cloned().unwrap_or_default();

    let tracks = tracks
        .into_iter()
        .filter_map(|track| {
            let id = track["id"].as_i64()?;
            let name = track["name"].as_str().unwrap_or("未知歌曲").to_string();
            let artists = track["ar"]
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
        })
        .collect();

    Ok(PlaylistDetailData {
        id,
        name,
        creator_name,
        track_count,
        tracks,
    })
}
