use ame_netease::NeteaseClient;
use ame_netease::api::track::lyric::TrackLyricRequest;
use ame_netease::api::track::url::TrackUrlRequest;
use anyhow::Result;

use crate::entity::player::{PlaybackMode, PlayerEntity};

pub fn set_mode(player: &mut PlayerEntity, mode: PlaybackMode) {
    player.set_mode(mode);
}

pub fn next(player: &mut PlayerEntity) -> Option<usize> {
    player.next_index()
}

pub fn prev(player: &mut PlayerEntity) -> Option<usize> {
    player.prev_index()
}

pub async fn fetch_track_url(cookie: &str, track_id: i64) -> Result<serde_json::Value> {
    let client = NeteaseClient::with_cookie(cookie);
    Ok(client
        .eapi_request(TrackUrlRequest::new(vec![track_id]))
        .await?)
}

pub async fn fetch_lyric(cookie: &str, track_id: i64) -> Result<serde_json::Value> {
    let client = NeteaseClient::with_cookie(cookie);
    Ok(client.weapi_request(TrackLyricRequest::new(track_id)).await?)
}
