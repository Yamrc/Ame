use std::time::{SystemTime, UNIX_EPOCH};

use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::cache::{
    CacheClass, CacheKey, CacheLookup, CachePolicy, CacheScope, CacheValue,
};
use crate::domain::library as library_actions;
use crate::domain::session as auth;
use crate::domain::session::AuthLevel;
use crate::page::playlist::models::{PlaylistPage, PlaylistTrackRow, SessionLoadKey};

const PLAYLIST_CACHE_VERSION: u32 = 1;

pub fn ensure_playlist_page_loaded<C: nekowg::AppContext>(
    runtime: &AppRuntime,
    playlist_id: i64,
    cx: &mut C,
) -> Result<PlaylistPage, String> {
    let user_id = auth::auth_user_id(runtime, cx);
    match read_playlist_page_cache(runtime, playlist_id, user_id)? {
        CacheLookup::Fresh(value) | CacheLookup::Stale(value) => return Ok(value.value),
        CacheLookup::Miss => {}
    }

    let Some(cookie) = auth::ensure_auth_cookie(runtime, AuthLevel::Guest, cx) else {
        return Err("Missing auth credentials".to_string());
    };
    let value = fetch_and_store_playlist_page(runtime, playlist_id, user_id, &cookie)?;
    Ok(value.value)
}

pub fn fetch_playlist_page_payload(playlist_id: i64, cookie: &str) -> Result<PlaylistPage, String> {
    let detail = library_actions::fetch_playlist_detail_blocking(playlist_id, cookie)
        .map_err(|err| err.to_string())?;
    Ok(PlaylistPage {
        id: detail.id,
        name: detail.name,
        creator_name: detail.creator_name,
        track_count: detail.track_count,
        tracks: detail
            .tracks
            .into_iter()
            .map(PlaylistTrackRow::from)
            .collect(),
    })
}

pub fn read_playlist_page_cache(
    runtime: &AppRuntime,
    playlist_id: i64,
    user_id: Option<i64>,
) -> Result<CacheLookup<PlaylistPage>, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(CacheLookup::Miss);
    };
    let key = playlist_cache_key(playlist_id, user_id)?;
    cache.read_json(CacheClass::Geological, &key, CachePolicy::geological())
}

pub fn fetch_and_store_playlist_page(
    runtime: &AppRuntime,
    playlist_id: i64,
    user_id: Option<i64>,
    cookie: &str,
) -> Result<CacheValue<PlaylistPage>, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        let value = fetch_playlist_page_payload(playlist_id, cookie)?;
        return Ok(CacheValue {
            value,
            fetched_at_ms: now_millis(),
        });
    };
    let key = playlist_cache_key(playlist_id, user_id)?;
    let tags = vec![format!("playlist:{playlist_id}")];
    cache.fetch_and_store_json(
        CacheClass::Geological,
        &key,
        CachePolicy::geological(),
        &tags,
        || fetch_playlist_page_payload(playlist_id, cookie),
    )
}

pub fn store_playlist_page(
    runtime: &AppRuntime,
    playlist_id: i64,
    user_id: Option<i64>,
    page: &PlaylistPage,
) -> Result<u64, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(now_millis());
    };
    let key = playlist_cache_key(playlist_id, user_id)?;
    let tags = vec![format!("playlist:{playlist_id}")];
    cache.write_json(
        CacheClass::Geological,
        &key,
        CachePolicy::geological(),
        &tags,
        page,
    )
}

pub fn session_load_key<T>(runtime: &AppRuntime, cx: &Context<T>) -> SessionLoadKey {
    let session = runtime.session.read(cx);
    (
        session.auth_user_id,
        session
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
    )
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn playlist_cache_key(playlist_id: i64, user_id: Option<i64>) -> Result<CacheKey, String> {
    let scope = user_id.map(CacheScope::User).unwrap_or(CacheScope::Public);
    CacheKey::new(
        "playlist.detail",
        PLAYLIST_CACHE_VERSION,
        scope,
        &playlist_id,
    )
}
