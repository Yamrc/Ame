use std::time::{SystemTime, UNIX_EPOCH};

use ame_netease::api::track::like::LikeTrackRequest;
use ame_netease::api::track::likelist::LikedTrackListRequest;
use serde::{Deserialize, Serialize};

use crate::app::runtime::AppRuntime;
use crate::domain::cache::{CacheClass, CacheKey, CacheLookup, CachePolicy, CacheScope};
use crate::domain::library;
use crate::domain::runtime::{block_on, netease_client};

const FAVORITES_CACHE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavoritesSnapshot {
    pub user_id: i64,
    pub liked_playlist_id: Option<i64>,
    pub track_ids: Vec<i64>,
    pub fetched_at_ms: u64,
}

pub fn fetch_favorites_snapshot(user_id: i64, cookie: &str) -> Result<FavoritesSnapshot, String> {
    let client = netease_client(Some(cookie));
    let response = block_on(client.eapi_request(LikedTrackListRequest::new(user_id)))
        .map_err(|err| err.to_string())?;
    let mut track_ids = response.ids().to_vec();
    track_ids.sort_unstable();
    track_ids.dedup();

    let liked_playlist_id = library::fetch_user_playlists_blocking(user_id, cookie)
        .map_err(|err| err.to_string())?
        .into_iter()
        .find(|item| item.special_type == 5)
        .map(|item| item.id);

    Ok(FavoritesSnapshot {
        user_id,
        liked_playlist_id,
        track_ids,
        fetched_at_ms: now_millis(),
    })
}

pub fn set_track_like_blocking(track_id: i64, like: bool, cookie: &str) -> Result<u64, String> {
    let cookie = normalize_weapi_cookie(cookie);
    let csrf_token = cookie_value(&parse_cookie_pairs(&cookie), "__csrf").unwrap_or_default();
    let client = netease_client(Some(cookie.as_str()));
    let response =
        block_on(client.weapi_request(LikeTrackRequest::new(track_id, like, csrf_token)))
            .map_err(|err| err.to_string())?;
    if response.code != 200 {
        return Err(format!(
            "favorite request returned unexpected code {}",
            response.code
        ));
    }
    Ok(now_millis())
}

pub fn read_favorites_cache(
    runtime: &AppRuntime,
    user_id: i64,
) -> Result<CacheLookup<FavoritesSnapshot>, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(CacheLookup::Miss);
    };
    cache.read_json(
        CacheClass::Firework,
        &favorites_cache_key(user_id)?,
        CachePolicy::firework(),
    )
}

pub fn store_favorites_cache(
    runtime: &AppRuntime,
    user_id: i64,
    snapshot: &FavoritesSnapshot,
) -> Result<u64, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(snapshot.fetched_at_ms);
    };
    cache.write_json(
        CacheClass::Firework,
        &favorites_cache_key(user_id)?,
        CachePolicy::firework(),
        &[format!("user:{user_id}:favorites")],
        snapshot,
    )
}

pub fn invalidate_related_caches(
    runtime: &AppRuntime,
    user_id: i64,
    liked_playlist_id: Option<i64>,
) -> Result<(), String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(());
    };

    cache.invalidate_tag(CacheClass::Firework, &format!("user:{user_id}:library"))?;
    if let Some(playlist_id) = liked_playlist_id {
        cache.invalidate_tag(CacheClass::Geological, &format!("playlist:{playlist_id}"))?;
    }
    Ok(())
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn favorites_cache_key(user_id: i64) -> Result<CacheKey, String> {
    CacheKey::new(
        "favorites.tracks",
        FAVORITES_CACHE_VERSION,
        CacheScope::User(user_id),
        &(),
    )
}

fn normalize_weapi_cookie(raw_cookie: &str) -> String {
    let mut pairs = parse_cookie_pairs(raw_cookie);
    upsert_cookie(&mut pairs, "os", "pc");
    upsert_cookie(&mut pairs, "appver", "3.1.17.204416");
    upsert_cookie(
        &mut pairs,
        "osver",
        "Microsoft-Windows-10-Professional-build-19045-64bit",
    );
    upsert_cookie(&mut pairs, "channel", "netease");
    upsert_cookie(&mut pairs, "WEVNSM", "1.0.0");
    pairs
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn parse_cookie_pairs(raw_cookie: &str) -> Vec<(String, String)> {
    raw_cookie
        .split(';')
        .filter_map(|part| {
            let trimmed = part.trim();
            let (key, value) = trimmed.split_once('=')?;
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                return None;
            }
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

fn upsert_cookie(pairs: &mut Vec<(String, String)>, key: &str, value: &str) {
    if pairs
        .iter()
        .any(|(existing_key, existing_value)| existing_key == key && !existing_value.is_empty())
    {
        return;
    }
    pairs.push((key.to_string(), value.to_string()));
}

fn cookie_value(pairs: &[(String, String)], key: &str) -> Option<String> {
    pairs.iter().find_map(|(existing_key, value)| {
        (existing_key == key && !value.is_empty()).then_some(value.clone())
    })
}
