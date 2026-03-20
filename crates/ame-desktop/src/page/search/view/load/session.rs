use nekowg::Context;

use crate::app::runtime::AppRuntime;
use crate::domain::session as auth_actions;
use crate::page::search::view::{SearchPageView, SessionLoadKey};
use crate::page::state::DataSource;

pub(in crate::page::search::view::load) fn session_load_key(
    runtime: &AppRuntime,
    cx: &Context<SearchPageView>,
) -> SessionLoadKey {
    let session = runtime.session.read(cx);
    (
        session.auth_user_id,
        session
            .auth_bundle
            .music_u
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
    )
}

pub(in crate::page::search::view::load) fn data_source(
    this: &SearchPageView,
    cx: &mut Context<SearchPageView>,
) -> DataSource {
    if session_load_key(&this.runtime, cx).1 {
        DataSource::User
    } else {
        DataSource::Guest
    }
}

pub(in crate::page::search::view::load) fn auth_cookie(
    this: &SearchPageView,
    cx: &mut Context<SearchPageView>,
) -> Option<String> {
    let session = this.runtime.session.read(cx).clone();
    auth_actions::build_cookie_header(&session.auth_bundle)
}
