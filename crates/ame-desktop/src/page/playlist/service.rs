use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nekowg::Context;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::app::runtime::AppRuntime;
use crate::domain::library as library_actions;
use crate::domain::session as auth;
use crate::domain::session::AuthLevel;
use crate::page::playlist::models::{PlaylistPage, PlaylistTrackRow, SessionLoadKey};

const PLAYLIST_DETAIL_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry<T> {
    fetched_at_ms: u64,
    data: T,
}

pub fn ensure_playlist_page_loaded<C: nekowg::AppContext>(
    runtime: &AppRuntime,
    playlist_id: i64,
    cx: &mut C,
) -> Result<PlaylistPage, String> {
    if let Some((page, _)) =
        cached_playlist_page(runtime, playlist_id, auth::auth_user_id(runtime, cx))
    {
        return Ok(page);
    }

    let Some(cookie) = auth::ensure_auth_cookie(runtime, AuthLevel::Guest, cx) else {
        return Err("缺少鉴权凭据".to_string());
    };
    let page = fetch_playlist_page_payload(playlist_id, &cookie)?;
    if cache_playlist_page(runtime, playlist_id, auth::auth_user_id(runtime, cx), &page).is_none() {
        warn!(
            playlist_id,
            "playlist detail cache write failed while ensuring page load"
        );
    }
    Ok(page)
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

pub fn cached_playlist_page(
    runtime: &AppRuntime,
    playlist_id: i64,
    user_id: Option<i64>,
) -> Option<(PlaylistPage, u64)> {
    let cache_key = playlist_cache_key(playlist_id, user_id);
    let entry = read_cache::<PlaylistPage>(runtime, &cache_key, PLAYLIST_DETAIL_TTL)?;
    Some((entry.data, entry.fetched_at_ms))
}

pub fn cache_playlist_page(
    runtime: &AppRuntime,
    playlist_id: i64,
    user_id: Option<i64>,
    page: &PlaylistPage,
) -> Option<u64> {
    let cache_key = playlist_cache_key(playlist_id, user_id);
    write_cache(runtime, &cache_key, page)
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

fn cache_is_fresh(fetched_at_ms: u64, ttl: Duration) -> bool {
    let ttl_ms = ttl.as_millis() as u64;
    let now_ms = now_millis();
    now_ms.saturating_sub(fetched_at_ms) <= ttl_ms
}

fn read_cache<T: DeserializeOwned>(
    runtime: &AppRuntime,
    key: &str,
    ttl: Duration,
) -> Option<CacheEntry<T>> {
    let store = runtime.services.cache_store.as_ref()?;
    let entry: CacheEntry<T> = match store.get(key) {
        Ok(Some(value)) => value,
        Ok(None) => return None,
        Err(err) => {
            warn!(cache_key = key, error = %err, "playlist cache read failed");
            return None;
        }
    };
    if cache_is_fresh(entry.fetched_at_ms, ttl) {
        Some(entry)
    } else {
        None
    }
}

fn write_cache<T: Serialize>(runtime: &AppRuntime, key: &str, data: &T) -> Option<u64> {
    let store = runtime.services.cache_store.as_ref()?;
    let fetched_at_ms = now_millis();
    let entry = CacheEntry {
        fetched_at_ms,
        data,
    };
    match store.upsert(key, &entry) {
        Ok(()) => Some(fetched_at_ms),
        Err(err) => {
            warn!(cache_key = key, error = %err, "playlist cache write failed");
            None
        }
    }
}

fn playlist_cache_key(playlist_id: i64, user_id: Option<i64>) -> String {
    user_id
        .map(|user_id| format!("playlist.detail.{playlist_id}.user.{user_id}"))
        .unwrap_or_else(|| format!("playlist.detail.{playlist_id}"))
}
