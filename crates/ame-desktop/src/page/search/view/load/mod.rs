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

impl SearchPageView {
    pub(super) fn handle_session_change(&mut self, cx: &mut Context<Self>) {
        let session_key = session_load_key(&self.runtime, cx);
        if self.last_session_key == session_key {
            return;
        }
        self.last_session_key = session_key;
        self.clear_search_state(cx);
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
        cx: &mut Context<Self>,
    ) {
        if self.route.keyword != keyword || session_load_key(&self.runtime, cx) != session_key {
            return;
        }

        self.state.update(cx, |state, cx| {
            match result {
                Ok(overview) => {
                    state.overview_keyword = keyword;
                    state.overview.succeed(overview, Some(now_millis()));
                }
                Err(err) => {
                    state.overview_keyword = keyword;
                    state.overview.clear();
                    state.overview.fail(err);
                }
            }
            cx.notify();
        });
    }

    fn apply_type_result(
        &mut self,
        keyword: String,
        route_type: SearchRouteType,
        append: bool,
        session_key: SessionLoadKey,
        result: Result<SearchTypePayload, String>,
        cx: &mut Context<Self>,
    ) {
        if self.route.keyword != keyword
            || self.route.route_type != Some(route_type)
            || session_load_key(&self.runtime, cx) != session_key
        {
            return;
        }

        self.state.update(cx, |state, cx| {
            match (route_type, result) {
                (SearchRouteType::Artists, Ok(SearchTypePayload::Artists(page))) => {
                    apply_collection_result(&mut state.artists, keyword, page, append)
                }
                (SearchRouteType::Albums, Ok(SearchTypePayload::Albums(page))) => {
                    apply_collection_result(&mut state.albums, keyword, page, append)
                }
                (SearchRouteType::Tracks, Ok(SearchTypePayload::Tracks(page))) => {
                    apply_collection_result(&mut state.tracks, keyword, page, append)
                }
                (SearchRouteType::Playlists, Ok(SearchTypePayload::Playlists(page))) => {
                    apply_collection_result(&mut state.playlists, keyword, page, append)
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
