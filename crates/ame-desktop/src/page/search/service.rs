use crate::app::runtime::AppRuntime;
use crate::domain::cache::{CacheClass, CacheKey, CacheLookup, CachePolicy, CacheScope};
use crate::page::search::types::{SearchOverview, SearchRouteType, SearchTypePayload};
use crate::page::search::view::SessionLoadKey;

const SEARCH_CACHE_VERSION: u32 = 1;

pub fn read_overview_cache(
    runtime: &AppRuntime,
    session_key: SessionLoadKey,
    keyword: &str,
) -> Result<CacheLookup<SearchOverview>, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(CacheLookup::Miss);
    };
    cache.read_json(
        CacheClass::Weather,
        &overview_cache_key(session_key, keyword)?,
        CachePolicy::weather(),
    )
}

pub fn store_overview_cache(
    runtime: &AppRuntime,
    session_key: SessionLoadKey,
    keyword: &str,
    payload: &SearchOverview,
) -> Result<u64, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(crate::domain::cache::now_millis());
    };
    cache.write_json(
        CacheClass::Weather,
        &overview_cache_key(session_key, keyword)?,
        CachePolicy::weather(),
        &search_tags(session_key, keyword),
        payload,
    )
}

pub fn read_collection_cache(
    runtime: &AppRuntime,
    session_key: SessionLoadKey,
    keyword: &str,
    route_type: SearchRouteType,
    offset: u32,
    limit: u32,
) -> Result<CacheLookup<SearchTypePayload>, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(CacheLookup::Miss);
    };
    cache.read_json(
        CacheClass::Weather,
        &collection_cache_key(session_key, keyword, route_type, offset, limit)?,
        CachePolicy::weather(),
    )
}

pub fn store_collection_cache(
    runtime: &AppRuntime,
    session_key: SessionLoadKey,
    keyword: &str,
    route_type: SearchRouteType,
    offset: u32,
    limit: u32,
    payload: &SearchTypePayload,
) -> Result<u64, String> {
    let Some(cache) = runtime.services.network_cache.as_ref() else {
        return Ok(crate::domain::cache::now_millis());
    };
    let mut tags = search_tags(session_key, keyword);
    tags.push(format!("search:type:{}", route_type.label()));
    cache.write_json(
        CacheClass::Weather,
        &collection_cache_key(session_key, keyword, route_type, offset, limit)?,
        CachePolicy::weather(),
        &tags,
        payload,
    )
}

fn overview_cache_key(session_key: SessionLoadKey, keyword: &str) -> Result<CacheKey, String> {
    CacheKey::new(
        "search.overview",
        SEARCH_CACHE_VERSION,
        search_scope(session_key),
        &(keyword.trim(),),
    )
}

fn collection_cache_key(
    session_key: SessionLoadKey,
    keyword: &str,
    route_type: SearchRouteType,
    offset: u32,
    limit: u32,
) -> Result<CacheKey, String> {
    CacheKey::new(
        "search.collection",
        SEARCH_CACHE_VERSION,
        search_scope(session_key),
        &(keyword.trim(), route_type.label(), offset, limit),
    )
}

fn search_scope(session_key: SessionLoadKey) -> CacheScope {
    if session_key.1 {
        session_key
            .0
            .map(CacheScope::User)
            .unwrap_or(CacheScope::Guest)
    } else if session_key.0.is_some() {
        CacheScope::Guest
    } else {
        CacheScope::Public
    }
}

fn search_tags(session_key: SessionLoadKey, keyword: &str) -> Vec<String> {
    let mut tags = vec![
        "search".to_string(),
        format!("search:query:{}", keyword.trim()),
    ];
    if let Some(user_id) = session_key.0 {
        tags.push(format!("user:{user_id}:search"));
    }
    tags
}
