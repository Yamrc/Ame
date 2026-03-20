use nekowg::Context;
use tracing::debug;

use crate::page::search::fetch::fetch_search_type_payload;
use crate::page::search::state::{prepare_collection_load, should_skip_collection_load};
use crate::page::search::types::SearchRouteType;

use super::super::{SearchPageView, TYPE_PAGE_LIMIT};
use super::session::{auth_cookie, data_source, session_load_key};

pub(in crate::page::search::view) fn ensure_type_loaded(
    this: &mut SearchPageView,
    keyword: String,
    route_type: SearchRouteType,
    append: bool,
    cx: &mut Context<SearchPageView>,
) {
    let source = data_source(this, cx);
    let state = this.state.read(cx).clone();
    let current_len = match route_type {
        SearchRouteType::Artists => state.artists.items.data.len(),
        SearchRouteType::Albums => state.albums.items.data.len(),
        SearchRouteType::Tracks => state.tracks.items.data.len(),
        SearchRouteType::Playlists => state.playlists.items.data.len(),
    };
    let should_skip = match route_type {
        SearchRouteType::Artists => should_skip_collection_load(&state.artists, &keyword, append),
        SearchRouteType::Albums => should_skip_collection_load(&state.albums, &keyword, append),
        SearchRouteType::Tracks => should_skip_collection_load(&state.tracks, &keyword, append),
        SearchRouteType::Playlists => {
            should_skip_collection_load(&state.playlists, &keyword, append)
        }
    };
    if should_skip {
        return;
    }

    let cookie = auth_cookie(this, cx);
    this.state.update(cx, |state, cx| {
        match route_type {
            SearchRouteType::Artists => {
                prepare_collection_load(&mut state.artists, keyword.clone(), append, source)
            }
            SearchRouteType::Albums => {
                prepare_collection_load(&mut state.albums, keyword.clone(), append, source)
            }
            SearchRouteType::Tracks => {
                prepare_collection_load(&mut state.tracks, keyword.clone(), append, source)
            }
            SearchRouteType::Playlists => {
                prepare_collection_load(&mut state.playlists, keyword.clone(), append, source)
            }
        }
        cx.notify();
    });

    let offset = if append { current_len as u32 } else { 0 };
    let page = cx.entity().downgrade();
    let request_keyword = keyword.clone();
    let request_session_key = session_load_key(&this.runtime, cx);
    cx.spawn(async move |_, cx| {
        let result = cx
            .background_executor()
            .spawn(async move {
                fetch_search_type_payload(
                    &request_keyword,
                    route_type,
                    offset,
                    TYPE_PAGE_LIMIT,
                    cookie.as_deref(),
                )
            })
            .await;
        if let Err(err) = page.update(cx, |this, cx| {
            this.apply_type_result(keyword, route_type, append, request_session_key, result, cx)
        }) {
            debug!("search collection load dropped before apply: {err}");
        }
    })
    .detach();
}
