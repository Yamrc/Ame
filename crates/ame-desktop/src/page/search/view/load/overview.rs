use nekowg::Context;
use tracing::{debug, warn};

use crate::domain::cache::CacheLookup;
use crate::page::search::fetch::fetch_search_overview_payload;
use crate::page::search::service::read_overview_cache;

use super::super::SearchPageView;
use super::session::{auth_cookie, data_source, session_load_key};

pub(in crate::page::search::view) fn ensure_overview_loaded(
    this: &mut SearchPageView,
    keyword: String,
    cx: &mut Context<SearchPageView>,
) {
    let source = data_source(this, cx);
    let state = this.state.read(cx).clone();
    if state.overview.loading {
        return;
    }
    let request_session_key = session_load_key(&this.runtime, cx);
    let mut used_stale_cache = false;
    match read_overview_cache(&this.runtime, request_session_key, &keyword) {
        Ok(CacheLookup::Fresh(cached)) => {
            this.apply_overview_result(
                keyword,
                request_session_key,
                Ok(cached.value),
                Some(cached.fetched_at_ms),
                cx,
            );
            return;
        }
        Ok(CacheLookup::Stale(cached)) => {
            this.apply_overview_result(
                keyword.clone(),
                request_session_key,
                Ok(cached.value),
                Some(cached.fetched_at_ms),
                cx,
            );
            this.state.update(cx, |state, cx| {
                state.overview.revalidate();
                cx.notify();
            });
            used_stale_cache = true;
        }
        Ok(CacheLookup::Miss) => {}
        Err(err) => {
            warn!(error = %err, "search overview cache read failed");
        }
    }

    let cookie = auth_cookie(this, cx);
    if !used_stale_cache {
        this.state.update(cx, |state, cx| {
            state.overview_keyword = keyword.clone();
            state.overview.begin(source);
            cx.notify();
        });
    }

    let page = cx.entity().downgrade();
    let request_keyword = keyword.clone();
    cx.spawn(async move |_, cx| {
        let result = cx
            .background_executor()
            .spawn(
                async move { fetch_search_overview_payload(&request_keyword, cookie.as_deref()) },
            )
            .await;
        if let Err(err) = page.update(cx, |this, cx| {
            this.apply_overview_result(keyword, request_session_key, result, None, cx)
        }) {
            debug!("search overview load dropped before apply: {err}");
        }
    })
    .detach();
}
