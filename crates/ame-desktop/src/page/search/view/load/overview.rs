use nekowg::Context;
use tracing::debug;

use crate::page::search::fetch::fetch_search_overview_payload;

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
    if state.overview_keyword == keyword
        && state.overview.source == source
        && state.overview.fetched_at_ms.is_some()
    {
        return;
    }

    let cookie = auth_cookie(this, cx);
    this.state.update(cx, |state, cx| {
        state.overview_keyword = keyword.clone();
        state.overview.begin(source);
        cx.notify();
    });

    let page = cx.entity().downgrade();
    let request_keyword = keyword.clone();
    let request_session_key = session_load_key(&this.runtime, cx);
    cx.spawn(async move |_, cx| {
        let result = cx
            .background_executor()
            .spawn(
                async move { fetch_search_overview_payload(&request_keyword, cookie.as_deref()) },
            )
            .await;
        if let Err(err) = page.update(cx, |this, cx| {
            this.apply_overview_result(keyword, request_session_key, result, cx)
        }) {
            debug!("search overview load dropped before apply: {err}");
        }
    })
    .detach();
}
