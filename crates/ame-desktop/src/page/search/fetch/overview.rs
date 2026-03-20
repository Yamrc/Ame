use crate::domain::search;

use super::super::types::SearchOverview;
use super::map::map_search_overview;

pub fn fetch_search_overview_payload(
    query: &str,
    cookie: Option<&str>,
) -> Result<SearchOverview, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(SearchOverview::default());
    }

    let artists =
        search::search_artists_blocking(query, 0, 16, cookie).map_err(|err| err.to_string())?;
    let albums =
        search::search_albums_blocking(query, 0, 16, cookie).map_err(|err| err.to_string())?;
    let tracks =
        search::search_songs_blocking(query, 0, 16, cookie).map_err(|err| err.to_string())?;
    let playlists =
        search::search_playlists_blocking(query, 0, 16, cookie).map_err(|err| err.to_string())?;

    Ok(map_search_overview(artists, albums, tracks, playlists))
}
