use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::runtime::AppRuntime;
use crate::domain::cache::{CacheClass, CacheKey, CacheLookup, CachePolicy, CacheScope};
use crate::page::discover::models::DiscoverLoadResult;

const DISCOVER_CACHE_VERSION: u32 = 1;

pub fn fetch_discover_payload(cookie: &str) -> Result<DiscoverLoadResult, String> {
    let playlists = crate::domain::library::fetch_top_playlists_blocking(60, 0, cookie)
        .map_err(|err| err.to_string())?;
    Ok(DiscoverLoadResult {
        playlists,
        fetched_at_ms: now_millis(),
    })
}

pub fn read_discover_payload_cache(
    runtime: &AppRuntime,
    user_id: Option<i64>,
    has_user_token: bool,
) -> Result<CacheLookup<DiscoverLoadResult>, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(CacheLookup::Miss);
    };
    cache.read_json(
        CacheClass::Weather,
        &discover_cache_key(user_id, has_user_token)?,
        CachePolicy::weather(),
    )
}

pub fn store_discover_payload_cache(
    runtime: &AppRuntime,
    user_id: Option<i64>,
    has_user_token: bool,
    payload: &DiscoverLoadResult,
) -> Result<u64, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(now_millis());
    };
    let mut tags = vec!["discover".to_string()];
    if let Some(user_id) = user_id {
        tags.push(format!("user:{user_id}:discover"));
    }
    cache.write_json(
        CacheClass::Weather,
        &discover_cache_key(user_id, has_user_token)?,
        CachePolicy::weather(),
        &tags,
        payload,
    )
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn discover_cache_key(user_id: Option<i64>, has_user_token: bool) -> Result<CacheKey, String> {
    let scope = if has_user_token {
        user_id.map(CacheScope::User).unwrap_or(CacheScope::Guest)
    } else {
        CacheScope::Guest
    };
    CacheKey::new("discover.payload", DISCOVER_CACHE_VERSION, scope, &())
}
