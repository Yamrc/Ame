mod models;
mod parse;
mod query;

use ame_netease::api::search::query::SearchType;
use anyhow::Result;

pub use models::*;

pub fn search_songs_blocking(
    keyword: &str,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchPage<SearchSongItem>> {
    let response = query::execute_search(keyword, SearchType::Song, offset, limit, cookie)?;
    let mut items = response
        .result
        .songs
        .iter()
        .filter_map(parse::parse_song_item)
        .collect::<Vec<_>>();
    parse::backfill_song_covers(&mut items, cookie)?;
    Ok(SearchPage {
        has_more: query::compute_has_more(
            response.result.has_more,
            response.result.song_count,
            offset,
            items.len(),
        ),
        items,
    })
}

pub fn search_artists_blocking(
    keyword: &str,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchPage<SearchArtistItem>> {
    let response = query::execute_search(keyword, SearchType::Artist, offset, limit, cookie)?;
    let items = response
        .result
        .artists
        .iter()
        .filter_map(parse::parse_artist_item)
        .collect::<Vec<_>>();
    Ok(SearchPage {
        has_more: query::compute_has_more(
            response.result.has_more,
            response.result.artist_count,
            offset,
            items.len(),
        ),
        items,
    })
}

pub fn search_albums_blocking(
    keyword: &str,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchPage<SearchAlbumItem>> {
    let response = query::execute_search(keyword, SearchType::Album, offset, limit, cookie)?;
    let items = response
        .result
        .albums
        .iter()
        .filter_map(parse::parse_album_item)
        .collect::<Vec<_>>();
    Ok(SearchPage {
        has_more: query::compute_has_more(
            response.result.has_more,
            response.result.album_count,
            offset,
            items.len(),
        ),
        items,
    })
}

pub fn search_playlists_blocking(
    keyword: &str,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchPage<SearchPlaylistItem>> {
    let response = query::execute_search(keyword, SearchType::Playlist, offset, limit, cookie)?;
    let items = response
        .result
        .playlists
        .iter()
        .filter_map(parse::parse_playlist_item)
        .collect::<Vec<_>>();
    Ok(SearchPage {
        has_more: query::compute_has_more(
            response.result.has_more,
            response.result.playlist_count,
            offset,
            items.len(),
        ),
        items,
    })
}
