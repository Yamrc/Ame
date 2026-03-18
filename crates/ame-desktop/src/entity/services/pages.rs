use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nekowg::AppContext;
use rand::RngExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::action::{library_actions, search_actions};
use crate::entity::app::HomeArtistLanguage;
use crate::entity::pages::PlaylistPageState;
use crate::entity::runtime::AppRuntime;
use crate::view::{playlist, search};

use super::auth::{self, AuthLevel};

const PLAYLIST_DETAIL_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const HOME_TOPLIST_IDS: [i64; 5] = [19723756, 180106, 60198, 3812895, 60131];

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry<T> {
    fetched_at_ms: u64,
    data: T,
}

#[derive(Debug, Clone)]
pub struct LibraryLoadResult {
    pub playlists: Vec<library_actions::LibraryPlaylistItem>,
    pub liked_tracks: Vec<library_actions::PlaylistTrackItem>,
    pub liked_lyric_lines: Vec<String>,
    pub fetched_at_ms: u64,
}

#[derive(Debug, Clone)]
pub struct HomeLoadResult {
    pub recommend_playlists: Vec<library_actions::LibraryPlaylistItem>,
    pub recommend_artists: Vec<library_actions::ArtistItem>,
    pub new_albums: Vec<library_actions::AlbumItem>,
    pub toplists: Vec<library_actions::ToplistItem>,
    pub daily_tracks: Vec<library_actions::DailyTrackItem>,
    pub personal_fm: Option<library_actions::FmTrackItem>,
    pub fetched_at_ms: u64,
}

#[derive(Debug, Clone)]
pub struct DiscoverLoadResult {
    pub playlists: Vec<library_actions::LibraryPlaylistItem>,
    pub fetched_at_ms: u64,
}

pub fn reset_session_bound_pages<C: AppContext>(runtime: &AppRuntime, cx: &mut C) {
    runtime.library.update(cx, |library, cx| {
        library.playlists.clear();
        library.liked_tracks.clear();
        library.liked_lyric_lines.clear();
        cx.notify();
    });
    runtime.playlist.update(cx, |playlist, cx| {
        playlist.pages.clear();
        cx.notify();
    });
    runtime.home.update(cx, |home, cx| {
        home.recommend_playlists.clear();
        home.recommend_artists.clear();
        home.new_albums.clear();
        home.toplists.clear();
        home.daily_tracks.clear();
        home.personal_fm.clear();
        cx.notify();
    });
    runtime.discover.update(cx, |discover, cx| {
        discover.playlists.clear();
        cx.notify();
    });
}

pub fn fetch_library_payload(user_id: i64, cookie: &str) -> Result<LibraryLoadResult, String> {
    let playlists = library_actions::fetch_user_playlists_blocking(user_id, cookie)
        .map_err(|err| err.to_string())?;

    let liked_id = playlists
        .iter()
        .find(|item| item.special_type == 5)
        .map(|item| item.id);

    let mut liked_tracks = Vec::new();
    let mut liked_lyric_lines = Vec::new();

    if let Some(liked_id) = liked_id {
        let detail = library_actions::fetch_playlist_detail_blocking(liked_id, cookie)
            .map_err(|err| err.to_string())?;
        let tracks = detail.tracks;
        liked_tracks = tracks.clone().into_iter().take(12).collect();
        if !tracks.is_empty() {
            let mut rng = rand::rng();
            let index = rng.random_range(0..tracks.len());
            let track_id = tracks[index].id;
            liked_lyric_lines =
                library_actions::fetch_track_lyric_preview_blocking(track_id, cookie)
                    .unwrap_or_default();
        }
    }

    Ok(LibraryLoadResult {
        playlists,
        liked_tracks,
        liked_lyric_lines,
        fetched_at_ms: now_millis(),
    })
}

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

fn pick_home_toplists(
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

pub fn fetch_discover_payload(cookie: &str) -> Result<DiscoverLoadResult, String> {
    let playlists = library_actions::fetch_top_playlists_blocking(60, 0, cookie)
        .map_err(|err| err.to_string())?;
    Ok(DiscoverLoadResult {
        playlists,
        fetched_at_ms: now_millis(),
    })
}

pub fn fetch_daily_tracks_payload(
    cookie: &str,
) -> Result<Vec<library_actions::DailyTrackItem>, String> {
    library_actions::fetch_daily_recommend_tracks_blocking(cookie).map_err(|err| err.to_string())
}

pub fn fetch_search_overview_payload(
    query: &str,
    cookie: Option<&str>,
) -> Result<search::SearchOverview, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(search::SearchOverview::default());
    }

    let artists = search_actions::search_artists_blocking(query, 0, 16, cookie)
        .map_err(|err| err.to_string())?;
    let albums = search_actions::search_albums_blocking(query, 0, 16, cookie)
        .map_err(|err| err.to_string())?;
    let tracks = search_actions::search_songs_blocking(query, 0, 16, cookie)
        .map_err(|err| err.to_string())?;
    let playlists = search_actions::search_playlists_blocking(query, 0, 16, cookie)
        .map_err(|err| err.to_string())?;

    Ok(search::SearchOverview {
        artists: artists.items.into_iter().map(map_search_artist).collect(),
        albums: albums.items.into_iter().map(map_search_album).collect(),
        tracks: tracks.items.into_iter().map(map_search_song).collect(),
        playlists: playlists
            .items
            .into_iter()
            .map(map_search_playlist)
            .collect(),
    })
}

pub fn fetch_search_type_payload(
    query: &str,
    route_type: search::SearchRouteType,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<search::SearchTypePayload, String> {
    let query = query.trim();
    if query.is_empty() {
        return match route_type {
            search::SearchRouteType::Artists => Ok(search::SearchTypePayload::Artists(
                search::SearchPageSlice {
                    items: Vec::new(),
                    has_more: false,
                },
            )),
            search::SearchRouteType::Albums => {
                Ok(search::SearchTypePayload::Albums(search::SearchPageSlice {
                    items: Vec::new(),
                    has_more: false,
                }))
            }
            search::SearchRouteType::Tracks => {
                Ok(search::SearchTypePayload::Tracks(search::SearchPageSlice {
                    items: Vec::new(),
                    has_more: false,
                }))
            }
            search::SearchRouteType::Playlists => Ok(search::SearchTypePayload::Playlists(
                search::SearchPageSlice {
                    items: Vec::new(),
                    has_more: false,
                },
            )),
        };
    }

    match route_type {
        search::SearchRouteType::Artists => {
            let page = search_actions::search_artists_blocking(query, offset, limit, cookie)
                .map_err(|err| err.to_string())?;
            Ok(search::SearchTypePayload::Artists(
                search::SearchPageSlice {
                    items: page.items.into_iter().map(map_search_artist).collect(),
                    has_more: page.has_more,
                },
            ))
        }
        search::SearchRouteType::Albums => {
            let page = search_actions::search_albums_blocking(query, offset, limit, cookie)
                .map_err(|err| err.to_string())?;
            Ok(search::SearchTypePayload::Albums(search::SearchPageSlice {
                items: page.items.into_iter().map(map_search_album).collect(),
                has_more: page.has_more,
            }))
        }
        search::SearchRouteType::Tracks => {
            let page = search_actions::search_songs_blocking(query, offset, limit, cookie)
                .map_err(|err| err.to_string())?;
            Ok(search::SearchTypePayload::Tracks(search::SearchPageSlice {
                items: page.items.into_iter().map(map_search_song).collect(),
                has_more: page.has_more,
            }))
        }
        search::SearchRouteType::Playlists => {
            let page = search_actions::search_playlists_blocking(query, offset, limit, cookie)
                .map_err(|err| err.to_string())?;
            Ok(search::SearchTypePayload::Playlists(
                search::SearchPageSlice {
                    items: page.items.into_iter().map(map_search_playlist).collect(),
                    has_more: page.has_more,
                },
            ))
        }
    }
}

fn map_search_song(item: search_actions::SearchSongItem) -> search::SearchSong {
    search::SearchSong {
        id: item.id,
        name: item.name,
        alias: item.alias,
        artists: item.artists,
        album: item.album,
        duration_ms: item.duration_ms,
        cover_url: item.cover_url,
    }
}

fn map_search_artist(item: search_actions::SearchArtistItem) -> search::SearchArtist {
    search::SearchArtist {
        id: item.id,
        name: item.name,
        cover_url: item.cover_url,
    }
}

fn map_search_album(item: search_actions::SearchAlbumItem) -> search::SearchAlbum {
    search::SearchAlbum {
        id: item.id,
        name: item.name,
        artist_name: item.artist_name,
        cover_url: item.cover_url,
    }
}

fn map_search_playlist(item: search_actions::SearchPlaylistItem) -> search::SearchPlaylist {
    search::SearchPlaylist {
        id: item.id,
        name: item.name,
        creator_name: if item.track_count == 0 {
            item.creator_name
        } else {
            format!("{} 首 · {}", item.track_count, item.creator_name)
        },
        track_count: item.track_count,
        cover_url: item.cover_url,
    }
}

pub fn ensure_playlist_page_loaded<C: AppContext>(
    runtime: &AppRuntime,
    playlist_id: i64,
    cx: &mut C,
) -> Result<playlist::PlaylistPage, String> {
    let mut playlist_state = playlist_state(runtime, cx);
    if let Some(page) = playlist_state.pages.data.get(&playlist_id).cloned() {
        return Ok(page);
    }

    if let Some((page, fetched_at_ms)) =
        cached_playlist_page(runtime, playlist_id, auth::auth_user_id(runtime, cx))
    {
        playlist_state.pages.data.insert(playlist_id, page.clone());
        playlist_state.pages.fetched_at_ms = Some(fetched_at_ms);
        set_playlist_state(runtime, playlist_state, cx);
        return Ok(page);
    }

    let Some(cookie) = auth::ensure_auth_cookie(runtime, AuthLevel::Guest, cx) else {
        return Err("缺少鉴权凭据".to_string());
    };
    let page = fetch_playlist_page_payload(playlist_id, &cookie)?;
    playlist_state.pages.data.insert(playlist_id, page.clone());
    playlist_state.pages.fetched_at_ms =
        cache_playlist_page(runtime, playlist_id, auth::auth_user_id(runtime, cx), &page);
    set_playlist_state(runtime, playlist_state, cx);
    Ok(page)
}

pub fn fetch_playlist_page_payload(
    playlist_id: i64,
    cookie: &str,
) -> Result<playlist::PlaylistPage, String> {
    let detail = library_actions::fetch_playlist_detail_blocking(playlist_id, cookie)
        .map_err(|err| err.to_string())?;
    Ok(playlist::PlaylistPage {
        id: detail.id,
        name: detail.name,
        creator_name: detail.creator_name,
        track_count: detail.track_count,
        tracks: detail
            .tracks
            .into_iter()
            .map(|track| playlist::PlaylistTrackRow {
                id: track.id,
                name: track.name,
                alias: track.alias,
                artists: track.artists,
                album: track.album,
                duration_ms: track.duration_ms,
                cover_url: track.cover_url,
            })
            .collect(),
    })
}

pub fn cached_playlist_page(
    runtime: &AppRuntime,
    playlist_id: i64,
    user_id: Option<i64>,
) -> Option<(playlist::PlaylistPage, u64)> {
    let cache_key = playlist_cache_key(playlist_id, user_id);
    let entry = read_cache::<playlist::PlaylistPage>(runtime, &cache_key, PLAYLIST_DETAIL_TTL)?;
    Some((entry.data, entry.fetched_at_ms))
}

pub fn cache_playlist_page(
    runtime: &AppRuntime,
    playlist_id: i64,
    user_id: Option<i64>,
    page: &playlist::PlaylistPage,
) -> Option<u64> {
    let cache_key = playlist_cache_key(playlist_id, user_id);
    write_cache(runtime, &cache_key, page)
}

fn now_millis() -> u64 {
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
    let entry: CacheEntry<T> = store.get(key).ok().flatten()?;
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
    if store.upsert(key, &entry).is_ok() {
        Some(fetched_at_ms)
    } else {
        None
    }
}

fn playlist_cache_key(playlist_id: i64, user_id: Option<i64>) -> String {
    user_id
        .map(|user_id| format!("playlist.detail.{playlist_id}.user.{user_id}"))
        .unwrap_or_else(|| format!("playlist.detail.{playlist_id}"))
}

fn playlist_state<C: AppContext>(runtime: &AppRuntime, cx: &C) -> PlaylistPageState {
    runtime
        .playlist
        .read_with(cx, |playlist, _| playlist.clone())
}

fn set_playlist_state<C: AppContext>(runtime: &AppRuntime, state: PlaylistPageState, cx: &mut C) {
    runtime.playlist.update(cx, |playlist, cx| {
        *playlist = state;
        cx.notify();
    });
}

#[cfg(test)]
mod tests {
    use super::pick_home_toplists;
    use crate::action::library_actions::ToplistItem;

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
