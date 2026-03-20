use ame_netease::NeteaseClient;
use ame_netease::api::playlist::list::PlaylistListRequest;
use ame_netease::api::playlist::personalized::PersonalizedPlaylistRequest;
use ame_netease::api::playlist::recommend_resource::RecommendResourceRequest;
use ame_netease::api::playlist::recommend_songs::RecommendSongsRequest;
use ame_netease::api::radio::personal_fm::PersonalFmRequest;
use ame_netease::api::user::playlist::UserPlaylistRequest;
use anyhow::Result;

use crate::domain::runtime::block_on;

use super::models::{DailyTrackItem, FmTrackItem, LibraryPlaylistItem};
use super::parse;

pub fn fetch_user_playlists_blocking(
    user_id: i64,
    cookie: &str,
) -> Result<Vec<LibraryPlaylistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(UserPlaylistRequest::new(user_id)))?;

    Ok(response
        .playlists
        .iter()
        .filter_map(|item| parse::parse_playlist_item(item, 256))
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
        .filter_map(|item| parse::parse_playlist_item(item, 1024))
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
        .filter_map(|item| parse::parse_playlist_item(item, 256))
        .collect())
}

pub fn fetch_daily_recommend_playlists_blocking(cookie: &str) -> Result<Vec<LibraryPlaylistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(RecommendResourceRequest::new()))?;
    Ok(response
        .playlists
        .iter()
        .filter_map(|item| parse::parse_playlist_item(item, 256))
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
    Ok(tracks
        .iter()
        .filter_map(parse::parse_daily_track_item)
        .collect())
}

pub fn fetch_personal_fm_blocking(cookie: &str) -> Result<Option<FmTrackItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(PersonalFmRequest::new()))?;
    Ok(response.data.iter().find_map(parse::parse_fm_track_item))
}
