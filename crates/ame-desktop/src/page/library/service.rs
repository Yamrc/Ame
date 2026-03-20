use std::time::{SystemTime, UNIX_EPOCH};

use rand::RngExt;

use crate::page::library::models::LibraryLoadResult;

pub fn fetch_library_payload(user_id: i64, cookie: &str) -> Result<LibraryLoadResult, String> {
    let playlists = crate::domain::library::fetch_user_playlists_blocking(user_id, cookie)
        .map_err(|err| err.to_string())?;

    let liked_id = playlists
        .iter()
        .find(|item| item.special_type == 5)
        .map(|item| item.id);

    let mut liked_tracks = Vec::new();
    let mut liked_lyric_lines = Vec::new();

    if let Some(liked_id) = liked_id {
        let detail = crate::domain::library::fetch_playlist_detail_blocking(liked_id, cookie)
            .map_err(|err| err.to_string())?;
        let tracks = detail.tracks;
        liked_tracks = tracks.clone().into_iter().take(12).collect();
        if !tracks.is_empty() {
            let mut rng = rand::rng();
            let index = rng.random_range(0..tracks.len());
            let track_id = tracks[index].id;
            liked_lyric_lines =
                crate::domain::library::fetch_track_lyric_preview_blocking(track_id, cookie)
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

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
