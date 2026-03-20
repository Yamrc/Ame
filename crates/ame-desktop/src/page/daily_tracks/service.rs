use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::library::DailyTrackItem;

pub fn fetch_daily_tracks_payload(cookie: &str) -> Result<Vec<DailyTrackItem>, String> {
    crate::domain::library::fetch_daily_recommend_tracks_blocking(cookie)
        .map_err(|err| err.to_string())
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
