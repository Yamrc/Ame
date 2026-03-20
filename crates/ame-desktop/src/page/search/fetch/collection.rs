use crate::domain::search;

use super::super::types::{SearchPageSlice, SearchRouteType, SearchTypePayload};
use super::map::{
    empty_payload, map_search_album, map_search_artist, map_search_playlist, map_search_song,
};

pub fn fetch_search_type_payload(
    query: &str,
    route_type: SearchRouteType,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchTypePayload, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(empty_payload(route_type));
    }

    match route_type {
        SearchRouteType::Artists => {
            let page = search::search_artists_blocking(query, offset, limit, cookie)
                .map_err(|err| err.to_string())?;
            Ok(SearchTypePayload::Artists(SearchPageSlice {
                items: page.items.into_iter().map(map_search_artist).collect(),
                has_more: page.has_more,
            }))
        }
        SearchRouteType::Albums => {
            let page = search::search_albums_blocking(query, offset, limit, cookie)
                .map_err(|err| err.to_string())?;
            Ok(SearchTypePayload::Albums(SearchPageSlice {
                items: page.items.into_iter().map(map_search_album).collect(),
                has_more: page.has_more,
            }))
        }
        SearchRouteType::Tracks => {
            let page = search::search_songs_blocking(query, offset, limit, cookie)
                .map_err(|err| err.to_string())?;
            Ok(SearchTypePayload::Tracks(SearchPageSlice {
                items: page.items.into_iter().map(map_search_song).collect(),
                has_more: page.has_more,
            }))
        }
        SearchRouteType::Playlists => {
            let page = search::search_playlists_blocking(query, offset, limit, cookie)
                .map_err(|err| err.to_string())?;
            Ok(SearchTypePayload::Playlists(SearchPageSlice {
                items: page.items.into_iter().map(map_search_playlist).collect(),
                has_more: page.has_more,
            }))
        }
    }
}
