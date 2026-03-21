use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::runtime::AppRuntime;
use crate::domain::cache::{CacheClass, CacheKey, CacheLookup, CachePolicy, CacheScope};
use crate::domain::library::DailyTrackItem;

const DAILY_TRACKS_CACHE_VERSION: u32 = 1;

pub fn fetch_daily_tracks_payload(cookie: &str) -> Result<Vec<DailyTrackItem>, String> {
    crate::domain::library::fetch_daily_recommend_tracks_blocking(cookie)
        .map_err(|err| err.to_string())
}

pub fn read_daily_tracks_cache(
    runtime: &AppRuntime,
    user_id: i64,
) -> Result<CacheLookup<Vec<DailyTrackItem>>, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(CacheLookup::Miss);
    };
    cache.read_json(
        CacheClass::Weather,
        &daily_tracks_cache_key(user_id)?,
        CachePolicy::weather(),
    )
}

pub fn store_daily_tracks_cache(
    runtime: &AppRuntime,
    user_id: i64,
    payload: &[DailyTrackItem],
) -> Result<u64, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(now_millis());
    };
    cache.write_json(
        CacheClass::Weather,
        &daily_tracks_cache_key(user_id)?,
        CachePolicy::weather(),
        &[
            format!("user:{user_id}:daily-tracks"),
            "daily-tracks".to_string(),
        ],
        payload,
    )
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn daily_tracks_cache_key(user_id: i64) -> Result<CacheKey, String> {
    CacheKey::new(
        "daily-tracks.payload",
        DAILY_TRACKS_CACHE_VERSION,
        CacheScope::User(user_id),
        &(),
    )
}
