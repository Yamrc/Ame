use std::collections::HashMap;

use nekowg::{App, AppContext, BorrowAppContext, Context, Global, SharedString};

use crate::app::route::{AppRoute, SearchCollectionKind};

pub type RouteParams = HashMap<SharedString, SharedString>;

#[derive(Debug, Clone)]
pub struct Location {
    pub pathname: SharedString,
}

#[derive(Debug, Clone)]
pub struct RouterState {
    pub location: Location,
    pub params: RouteParams,
    pub route: AppRoute,
}

impl Default for RouterState {
    fn default() -> Self {
        Self {
            location: Location {
                pathname: "/".into(),
            },
            params: RouteParams::default(),
            route: AppRoute::Home,
        }
    }
}

impl Global for RouterState {}

pub fn init(cx: &mut App) {
    cx.update_default_global(|state: &mut RouterState, _| {
        state.route = AppRoute::parse(state.location.pathname.as_ref());
        state.params = route_params(&state.route);
        state.location.pathname = state.route.to_path();
    });
}

pub fn current_route<T>(cx: &mut Context<T>) -> AppRoute {
    cx.read_global(|state: &RouterState, _| state.route.clone())
}

pub fn navigate_route<T>(cx: &mut Context<T>, route: AppRoute) {
    cx.update_global(|state: &mut RouterState, _| {
        state.route = route;
        state.location.pathname = state.route.to_path();
        state.params = route_params(&state.route);
    });
}

fn route_params(route: &AppRoute) -> RouteParams {
    let mut params = RouteParams::default();
    match route {
        AppRoute::SearchOverview { query } => {
            params.insert("keywords".into(), SharedString::from(query.clone()));
        }
        AppRoute::SearchCollection { query, kind } => {
            params.insert("keywords".into(), SharedString::from(query.clone()));
            params.insert(
                "type".into(),
                SharedString::from(match kind {
                    SearchCollectionKind::Artists => "artists",
                    SearchCollectionKind::Albums => "albums",
                    SearchCollectionKind::Tracks => "tracks",
                    SearchCollectionKind::Playlists => "playlists",
                }),
            );
        }
        AppRoute::Playlist { id } => {
            params.insert("id".into(), SharedString::from(id.to_string()));
        }
        _ => {}
    }
    params
}
