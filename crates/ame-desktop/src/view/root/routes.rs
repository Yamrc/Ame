use nekowg::{AnyElement, Context, Entity, IntoElement};

use crate::router::{Route, Routes};
use crate::view::{daily_tracks, discover, home, library, login, next, playlist, search, settings};

use super::RootView;

pub(super) struct RootPages {
    pub home: Entity<home::HomePageView>,
    pub discover: Entity<discover::DiscoverPageView>,
    pub library: Entity<library::LibraryPageView>,
    pub search: Entity<search::SearchPageView>,
    pub daily_tracks: Entity<daily_tracks::DailyTracksPageView>,
    pub next: Entity<next::NextPageView>,
    pub playlist: Entity<playlist::PlaylistPageView>,
    pub settings: Entity<settings::SettingsPageView>,
    pub login: Entity<login::LoginPageView>,
}

impl RootView {
    pub(super) fn render_routes(&self, cx: &mut Context<Self>) -> AnyElement {
        Routes::new()
            .basename("/")
            .child(Route::new().index().element({
                let page = self.pages.home.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .child(Route::new().path("explore").element({
                let page = self.pages.discover.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .child(Route::new().path("library").element({
                let page = self.pages.library.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .child(Route::new().path("search").element({
                let page = self.pages.search.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .child(Route::new().path("search/{keywords}").element({
                let page = self.pages.search.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .child(Route::new().path("daily/songs").element({
                let page = self.pages.daily_tracks.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .child(Route::new().path("next").element({
                let page = self.pages.next.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .child(Route::new().path("playlist/{id}").element({
                let page = self.pages.playlist.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .child(Route::new().path("settings").element({
                let page = self.pages.settings.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .child(Route::new().path("login").element({
                let page = self.pages.login.clone();
                move |_, _| page.clone().into_any_element()
            }))
            .render(cx)
    }
}
