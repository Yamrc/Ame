use ame_netease::NeteaseClient;
use ame_netease::api::track::lyric::TrackLyricRequest;
use anyhow::Result;
use rand::RngExt;

use crate::domain::runtime::block_on;

use super::parse;

pub fn fetch_track_lyric_preview_blocking(track_id: i64, cookie: &str) -> Result<Vec<String>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(TrackLyricRequest::new(track_id)))?;
    let raw = response.main_lyric().unwrap_or_default();
    let mut lines = parse::parse_lyric_lines(raw);
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
