use ame_netease::NeteaseClient;
use ame_netease::api::album::new::AlbumNewRequest;
use ame_netease::api::artist::toplist::ToplistArtistRequest;
use ame_netease::api::playlist::toplist::ToplistRequest;
use anyhow::Result;

use crate::domain::runtime::block_on;

use super::models::{AlbumItem, ArtistItem, ToplistItem};
use super::parse;

pub fn fetch_recommend_artists_blocking(
    artist_type: u32,
    limit: u32,
    cookie: &str,
) -> Result<Vec<ArtistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let pool = limit.max(60);
    let response = block_on(client.weapi_request(ToplistArtistRequest::new(artist_type, pool, 0)))?;
    let mut items = response
        .list
        .artists
        .iter()
        .chain(response.artists.iter())
        .filter_map(parse::parse_artist_item)
        .collect::<Vec<_>>();

    let target = limit as usize;
    if items.len() < target {
        let offset = items.len() as u32;
        let response =
            block_on(client.weapi_request(ToplistArtistRequest::new(artist_type, pool, offset)))?;
        for artist in response
            .list
            .artists
            .iter()
            .chain(response.artists.iter())
            .filter_map(parse::parse_artist_item)
        {
            if items.iter().any(|existing| existing.id == artist.id) {
                continue;
            }
            items.push(artist);
            if items.len() >= target {
                break;
            }
        }
    }

    if items.len() > 1 {
        use std::time::{SystemTime, UNIX_EPOCH};
        let mut seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        for i in (1..items.len()).rev() {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let j = (seed as usize) % (i + 1);
            items.swap(i, j);
        }
    }

    if items.len() > target {
        items.truncate(target);
    }

    Ok(items)
}

pub fn fetch_new_albums_blocking(
    limit: u32,
    offset: u32,
    area: &str,
    cookie: &str,
) -> Result<Vec<AlbumItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.weapi_request(AlbumNewRequest::new(limit, offset, area)))?;
    Ok(response
        .albums
        .iter()
        .filter_map(parse::parse_album_item)
        .collect())
}

pub fn fetch_toplists_blocking(cookie: &str) -> Result<Vec<ToplistItem>> {
    let client = NeteaseClient::with_cookie(cookie);
    let response = block_on(client.eapi_request(ToplistRequest::new()))?;
    Ok(response
        .list
        .iter()
        .filter_map(parse::parse_toplist_item)
        .collect())
}
