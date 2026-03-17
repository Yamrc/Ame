use ame_netease::api::search::song::SearchSongRequest;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSongItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
}

use crate::action::runtime::{block_on, netease_client};

pub fn search_song_blocking(keyword: &str, cookie: Option<&str>) -> Result<Vec<SearchSongItem>> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Ok(Vec::new());
    }

    let client = netease_client(cookie);
    let response = block_on(client.weapi_request(SearchSongRequest::new(keyword)))?;

    let parsed = response
        .result
        .songs
        .into_iter()
        .filter(|song| song.id > 0)
        .map(|song| {
            let alias = song
                .tns
                .into_iter()
                .chain(song.alia)
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>()
                .join(" / ");
            let artists = song
                .artists
                .into_iter()
                .filter_map(|artist| artist.name)
                .filter(|name| !name.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" / ");

            SearchSongItem {
                id: song.id,
                name: song.name.unwrap_or_default(),
                alias: (!alias.is_empty()).then_some(alias),
                artists: if artists.is_empty() {
                    "未知艺人".to_string()
                } else {
                    artists
                },
                album: song.album.name.filter(|value| !value.trim().is_empty()),
                duration_ms: song.duration_ms,
            }
        })
        .collect();

    Ok(parsed)
}
