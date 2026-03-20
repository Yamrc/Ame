use std::time::{SystemTime, UNIX_EPOCH};

use crate::page::discover::models::DiscoverLoadResult;

pub fn fetch_discover_payload(cookie: &str) -> Result<DiscoverLoadResult, String> {
    let playlists = crate::domain::library::fetch_top_playlists_blocking(60, 0, cookie)
        .map_err(|err| err.to_string())?;
    Ok(DiscoverLoadResult {
        playlists,
        fetched_at_ms: now_millis(),
    })
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
