use std::time::{SystemTime, UNIX_EPOCH};

use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::cache::{CacheClass, CacheKey, CacheLookup, CachePolicy, CacheScope};
use crate::domain::library as library_actions;
use crate::domain::session::SessionState;
use crate::domain::settings::HomeArtistLanguage;
use crate::page::home::models::{HomeLoadResult, HomeSessionKey};

const HOME_TOPLIST_IDS: [i64; 5] = [19723756, 180106, 60198, 3812895, 60131];
const HOME_CACHE_VERSION: u32 = 1;

pub fn fetch_home_payload(
    cookie: &str,
    is_user: bool,
    artist_language: HomeArtistLanguage,
) -> Result<HomeLoadResult, String> {
    let limit = 10;
    let mut recommend_playlists = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if is_user {
        let items = library_actions::fetch_daily_recommend_playlists_blocking(cookie)
            .map_err(|err| format!("failed to fetch daily recommend playlists: {err}"))?;
        for item in items {
            if seen.insert(item.id) {
                recommend_playlists.push(item);
            }
        }
    }

    for item in library_actions::fetch_personalized_playlists_blocking(limit, cookie)
        .map_err(|err| format!("failed to fetch personalized playlists: {err}"))?
    {
        if seen.insert(item.id) {
            recommend_playlists.push(item);
        }
    }
    if recommend_playlists.len() > limit as usize {
        recommend_playlists.truncate(limit as usize);
    }

    let recommend_artists = library_actions::fetch_recommend_artists_blocking(
        artist_language.toplist_type(),
        6,
        cookie,
    )
    .map_err(|err| format!("failed to fetch recommend artists: {err}"))?;
    let new_albums = library_actions::fetch_new_albums_blocking(10, 0, "ALL", cookie)
        .map_err(|err| format!("failed to fetch new albums: {err}"))?;
    let toplists = pick_home_toplists(
        library_actions::fetch_toplists_blocking(cookie)
            .map_err(|err| format!("failed to fetch toplists: {err}"))?,
    );
    let daily_tracks = if is_user {
        library_actions::fetch_daily_recommend_tracks_blocking(cookie)
            .map_err(|err| format!("failed to fetch daily tracks: {err}"))?
    } else {
        Vec::new()
    };
    let personal_fm = if is_user {
        library_actions::fetch_personal_fm_blocking(cookie)
            .map_err(|err| format!("failed to fetch personal fm: {err}"))?
    } else {
        None
    };

    Ok(HomeLoadResult {
        recommend_playlists,
        recommend_artists,
        new_albums,
        toplists,
        daily_tracks,
        personal_fm,
        fetched_at_ms: now_millis(),
    })
}

pub fn pick_home_toplists(
    toplists: Vec<library_actions::ToplistItem>,
) -> Vec<library_actions::ToplistItem> {
    let mut by_id = toplists
        .iter()
        .cloned()
        .map(|item| (item.id, item))
        .collect::<std::collections::HashMap<_, _>>();
    let mut picked = Vec::with_capacity(HOME_TOPLIST_IDS.len());
    for id in HOME_TOPLIST_IDS {
        if let Some(item) = by_id.remove(&id) {
            picked.push(item);
        }
    }
    if picked.is_empty() {
        toplists.into_iter().take(5).collect()
    } else {
        picked
    }
}

pub fn session_key<T>(runtime: &AppRuntime, cx: &Context<T>) -> HomeSessionKey {
    runtime
        .session
        .read_with(cx, |session, _| session_key_from_session(session))
}

pub fn session_key_from_session(session: &SessionState) -> HomeSessionKey {
    HomeSessionKey {
        user_id: session.auth_user_id,
        has_user_token: session
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        has_guest_token: session
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            || session
                .auth_bundle
                .music_a
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty()),
    }
}

pub fn read_home_payload_cache(
    runtime: &AppRuntime,
    key: HomeSessionKey,
    artist_language: HomeArtistLanguage,
) -> Result<CacheLookup<HomeLoadResult>, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(CacheLookup::Miss);
    };
    cache.read_json(
        CacheClass::Weather,
        &home_cache_key(key, artist_language)?,
        CachePolicy::weather(),
    )
}

pub fn store_home_payload_cache(
    runtime: &AppRuntime,
    key: HomeSessionKey,
    artist_language: HomeArtistLanguage,
    payload: &HomeLoadResult,
) -> Result<u64, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(now_millis());
    };
    let mut tags = vec![
        "home".to_string(),
        format!("home:artist-language:{:?}", artist_language),
    ];
    if let Some(user_id) = key.user_id {
        tags.push(format!("user:{user_id}:home"));
    }
    cache.write_json(
        CacheClass::Weather,
        &home_cache_key(key, artist_language)?,
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

fn home_cache_key(
    key: HomeSessionKey,
    artist_language: HomeArtistLanguage,
) -> Result<CacheKey, String> {
    let scope = if key.has_user_token {
        key.user_id
            .map(CacheScope::User)
            .unwrap_or(CacheScope::Guest)
    } else {
        CacheScope::Guest
    };
    CacheKey::new(
        "home.payload",
        HOME_CACHE_VERSION,
        scope,
        &(artist_language, key.has_user_token),
    )
}

#[cfg(test)]
mod tests {
    use super::pick_home_toplists;
    use crate::domain::library::ToplistItem;

    fn toplist(id: i64) -> ToplistItem {
        ToplistItem {
            id,
            name: format!("榜单{id}"),
            update_frequency: "每日".to_string(),
            cover_url: None,
        }
    }

    #[test]
    fn pick_home_toplists_uses_fixed_order() {
        let input = vec![
            toplist(60198),
            toplist(180106),
            toplist(19723756),
            toplist(3812895),
            toplist(60131),
            toplist(123),
        ];
        let picked = pick_home_toplists(input);
        let ids = picked.into_iter().map(|item| item.id).collect::<Vec<_>>();
        assert_eq!(ids, vec![19723756, 180106, 60198, 3812895, 60131]);
    }
}
