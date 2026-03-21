mod collection;
mod overview;
mod session;

use nekowg::Context;

use crate::page::search::state::{apply_collection_error, apply_collection_result, now_millis};
use crate::page::search::types::{SearchOverview, SearchRouteType, SearchTypePayload};
use crate::page::state::freeze_page_state;

use super::{SearchPageView, SessionLoadKey};

pub(super) use collection::ensure_type_loaded;
pub(super) use overview::ensure_overview_loaded;

struct TypeResultApply {
    keyword: String,
    route_type: SearchRouteType,
    append: bool,
    session_key: SessionLoadKey,
    request_offset: u32,
    fetched_at_ms: Option<u64>,
}

impl SearchPageView {
    pub(super) fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let session_key = session_load_key(&self.runtime, cx);
        if self.last_session_key == session_key {
            return;
        }
        self.last_session_key = session_key;
        if !self.active {
            self.clear_search_state(cx);
        }
        if self.active {
            self.ensure_loaded(cx);
        }
    }

    pub(super) fn clear_search_state(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| {
            state.clear_all();
            cx.notify();
        });
    }

    pub(super) fn release_search_heavy_data(&mut self, cx: &mut Context<Self>) {
        freeze_page_state(&self.state, cx);
    }

    pub(super) fn ensure_loaded(&mut self, cx: &mut Context<Self>) {
        let keyword = self.route.keyword.trim().to_string();
        if keyword.is_empty() {
            self.clear_search_state(cx);
            return;
        }

        match self.route.route_type {
            None => ensure_overview_loaded(self, keyword, cx),
            Some(route_type) => ensure_type_loaded(self, keyword, route_type, false, cx),
        }
    }

    fn apply_overview_result(
        &mut self,
        keyword: String,
        session_key: SessionLoadKey,
        result: Result<SearchOverview, String>,
        cached_fetched_at_ms: Option<u64>,
        cx: &mut Context<Self>,
    ) {
        if self.route.keyword != keyword || session_load_key(&self.runtime, cx) != session_key {
            return;
        }

        self.state.update(cx, |state, cx| {
            match result {
                Ok(overview) => {
                    let fetched_at_ms = cached_fetched_at_ms.unwrap_or_else(|| {
                        crate::page::search::service::store_overview_cache(
                            &self.runtime,
                            session_key,
                            &keyword,
                            &overview,
                        )
                        .unwrap_or_else(|_| now_millis())
                    });
                    state.overview_keyword = keyword;
                    state.overview.succeed(overview, Some(fetched_at_ms));
                }
                Err(err) => {
                    state.overview_keyword = keyword;
                    if state.overview.has_cached_value() {
                        state.overview.fail(err);
                    } else {
                        state.overview.clear();
                        state.overview.fail(err);
                    }
                }
            }
            cx.notify();
        });
    }

    fn apply_type_result(
        &mut self,
        apply: TypeResultApply,
        result: Result<SearchTypePayload, String>,
        cx: &mut Context<Self>,
    ) {
        let TypeResultApply {
            keyword,
            route_type,
            append,
            session_key,
            request_offset,
            fetched_at_ms: cached_fetched_at_ms,
        } = apply;
        if self.route.keyword != keyword
            || self.route.route_type != Some(route_type)
            || session_load_key(&self.runtime, cx) != session_key
        {
            return;
        }

        self.state.update(cx, |state, cx| {
            let fetched_at_ms = cached_fetched_at_ms.unwrap_or_else(|| match &result {
                Ok(payload) => crate::page::search::service::store_collection_cache(
                    &self.runtime,
                    session_key,
                    &keyword,
                    route_type,
                    request_offset,
                    super::TYPE_PAGE_LIMIT,
                    payload,
                )
                .unwrap_or_else(|_| now_millis()),
                Err(_) => now_millis(),
            });
            match (route_type, result) {
                (SearchRouteType::Artists, Ok(SearchTypePayload::Artists(page))) => {
                    apply_collection_result(
                        &mut state.artists,
                        keyword,
                        page,
                        append,
                        fetched_at_ms,
                    )
                }
                (SearchRouteType::Albums, Ok(SearchTypePayload::Albums(page))) => {
                    apply_collection_result(&mut state.albums, keyword, page, append, fetched_at_ms)
                }
                (SearchRouteType::Tracks, Ok(SearchTypePayload::Tracks(page))) => {
                    apply_collection_result(&mut state.tracks, keyword, page, append, fetched_at_ms)
                }
                (SearchRouteType::Playlists, Ok(SearchTypePayload::Playlists(page))) => {
                    apply_collection_result(
                        &mut state.playlists,
                        keyword,
                        page,
                        append,
                        fetched_at_ms,
                    )
                }
                (_, Ok(_)) => {}
                (SearchRouteType::Artists, Err(err)) => {
                    apply_collection_error(&mut state.artists, keyword, err, append)
                }
                (SearchRouteType::Albums, Err(err)) => {
                    apply_collection_error(&mut state.albums, keyword, err, append)
                }
                (SearchRouteType::Tracks, Err(err)) => {
                    apply_collection_error(&mut state.tracks, keyword, err, append)
                }
                (SearchRouteType::Playlists, Err(err)) => {
                    apply_collection_error(&mut state.playlists, keyword, err, append)
                }
            }
            cx.notify();
        });
    }

    pub(super) fn load_more(&mut self, cx: &mut Context<Self>) {
        if let Some(route_type) = self.route.route_type {
            ensure_type_loaded(self, self.route.keyword.clone(), route_type, true, cx);
        }
    }
}

pub(super) fn session_load_key(
    runtime: &crate::app::runtime::AppRuntime,
    cx: &Context<SearchPageView>,
) -> SessionLoadKey {
    session::session_load_key(runtime, cx)
}
