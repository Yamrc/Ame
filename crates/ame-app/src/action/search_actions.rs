use ame_netease::NeteaseClient;
use ame_netease::api::search::song::SearchSongRequest;
use anyhow::Result;

pub async fn search_song(cookie: &str, keyword: &str) -> Result<serde_json::Value> {
    let client = NeteaseClient::with_cookie(cookie);
    let req = SearchSongRequest::new(keyword);
    Ok(client.weapi_request(req).await?)
}
