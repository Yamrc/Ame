use ame_netease::api::search::song::SearchSongRequest;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSongItem {
    pub id: i64,
    pub name: String,
    pub artists: String,
}

use crate::action::runtime::{block_on, netease_client};

pub fn search_song_blocking(keyword: &str, cookie: Option<&str>) -> Result<Vec<SearchSongItem>> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Ok(Vec::new());
    }

    let client = netease_client(cookie);
    let response: serde_json::Value =
        block_on(client.weapi_request(SearchSongRequest::new(keyword)))?;
    let songs = response["result"]["songs"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let parsed = songs
        .into_iter()
        .filter_map(|song| {
            let id = song["id"].as_i64()?;
            let name = song["name"].as_str().unwrap_or("").to_string();
            let artists = song["artists"]
                .as_array()
                .map(|artists| {
                    artists
                        .iter()
                        .filter_map(|artist| artist["name"].as_str().map(ToString::to_string))
                        .collect::<Vec<String>>()
                        .join(" / ")
                })
                .filter(|artists| !artists.is_empty())
                .unwrap_or_else(|| "未知艺人".to_string());

            Some(SearchSongItem { id, name, artists })
        })
        .collect();

    Ok(parsed)
}
