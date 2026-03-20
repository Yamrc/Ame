use nekowg::{AppContext, Context, ScrollHandle};

use crate::app::page_host::PageHostView;
use crate::app::page_host::key::PageKey;
use crate::app::route::AppRoute;
use crate::app::runtime::AppRuntime;
use crate::page::{daily_tracks, discover, home, library, login, next, playlist, search, settings};

use super::PageSlot;
use super::unknown::UnknownPageView;

pub(in crate::app::page_host) fn create_page(
    runtime: &AppRuntime,
    page_scroll_handle: &ScrollHandle,
    key: &PageKey,
    route: &AppRoute,
    cx: &mut Context<PageHostView>,
) -> PageSlot {
    match key {
        PageKey::Home => {
            let runtime = runtime.clone();
            PageSlot::Home(cx.new(move |cx| home::HomePageView::new(runtime.clone(), cx)))
        }
        PageKey::Explore => {
            let runtime = runtime.clone();
            PageSlot::Discover(
                cx.new(move |cx| discover::DiscoverPageView::new(runtime.clone(), cx)),
            )
        }
        PageKey::Library => {
            let runtime = runtime.clone();
            PageSlot::Library(cx.new(move |cx| library::LibraryPageView::new(runtime.clone(), cx)))
        }
        PageKey::Search(search_route) => {
            let runtime = runtime.clone();
            let search_route = search_route.clone();
            PageSlot::Search(cx.new(move |cx| {
                search::SearchPageView::new(runtime.clone(), search_route.clone(), cx)
            }))
        }
        PageKey::DailyTracks => {
            let runtime = runtime.clone();
            PageSlot::DailyTracks(
                cx.new(move |cx| daily_tracks::DailyTracksPageView::new(runtime.clone(), cx)),
            )
        }
        PageKey::Queue => {
            let runtime = runtime.clone();
            let page_scroll_handle = page_scroll_handle.clone();
            PageSlot::Next(cx.new(move |cx| {
                next::NextPageView::new(runtime.clone(), page_scroll_handle.clone(), cx)
            }))
        }
        PageKey::Playlist(playlist_id) => {
            let runtime = runtime.clone();
            let page_scroll_handle = page_scroll_handle.clone();
            let playlist_id = *playlist_id;
            PageSlot::Playlist(cx.new(move |cx| {
                playlist::PlaylistPageView::new(
                    runtime.clone(),
                    page_scroll_handle.clone(),
                    playlist_id,
                    cx,
                )
            }))
        }
        PageKey::Settings => {
            let runtime = runtime.clone();
            PageSlot::Settings(
                cx.new(move |cx| settings::SettingsPageView::new(runtime.clone(), cx)),
            )
        }
        PageKey::Login => {
            let runtime = runtime.clone();
            PageSlot::Login(cx.new(move |cx| login::LoginPageView::new(runtime.clone(), cx)))
        }
        PageKey::Unknown(_) => {
            let path = match route {
                AppRoute::Unknown { path } => path.as_ref().to_string(),
                _ => route.to_path().as_ref().to_string(),
            };
            PageSlot::Unknown(cx.new(move |_| UnknownPageView { path: path.clone() }))
        }
    }
}
