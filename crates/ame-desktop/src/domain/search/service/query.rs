use ame_netease::api::search::query::{SearchRequest, SearchResponse, SearchType};
use anyhow::Result;

use crate::domain::runtime::{block_on, netease_client};

pub(super) fn execute_search(
    keyword: &str,
    search_type: SearchType,
    offset: u32,
    limit: u32,
    cookie: Option<&str>,
) -> Result<SearchResponse> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Ok(SearchResponse::default());
    }

    let client = netease_client(cookie);
    let mut request = SearchRequest::new(keyword, search_type);
    request.offset = offset;
    request.limit = limit;
    block_on(client.weapi_request(request))
}

pub(super) fn compute_has_more(
    explicit: bool,
    total: u64,
    offset: u32,
    current_len: usize,
) -> bool {
    explicit || total > u64::from(offset) + current_len as u64
}
