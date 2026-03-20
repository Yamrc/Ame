use crate::app::route::AppRoute;
use crate::page::search;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum PageKey {
    Home,
    Explore,
    Library,
    Search(search::SearchPageRoute),
    Playlist(i64),
    DailyTracks,
    Queue,
    Settings,
    Login,
    Unknown(String),
}

impl PageKey {
    pub(super) fn from_route(route: &AppRoute) -> Self {
        match route {
            AppRoute::Home => Self::Home,
            AppRoute::Explore => Self::Explore,
            AppRoute::Library => Self::Library,
            AppRoute::Search
            | AppRoute::SearchOverview { .. }
            | AppRoute::SearchCollection { .. } => {
                if let Some(search_route) = search::SearchPageRoute::from_app_route(route) {
                    Self::Search(search_route)
                } else {
                    Self::Unknown(route.to_path().as_ref().to_string())
                }
            }
            AppRoute::Playlist { id } => Self::Playlist(*id),
            AppRoute::DailyTracks => Self::DailyTracks,
            AppRoute::Queue => Self::Queue,
            AppRoute::Settings => Self::Settings,
            AppRoute::Login => Self::Login,
            AppRoute::Unknown { path } => Self::Unknown(path.as_ref().to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::route::{AppRoute, SearchCollectionKind};

    use super::PageKey;

    #[test]
    fn singleton_routes_map_to_stable_keys() {
        assert_eq!(PageKey::from_route(&AppRoute::Home), PageKey::Home);
        assert_eq!(PageKey::from_route(&AppRoute::Explore), PageKey::Explore);
        assert_eq!(PageKey::from_route(&AppRoute::Library), PageKey::Library);
        assert_eq!(PageKey::from_route(&AppRoute::Queue), PageKey::Queue);
        assert_eq!(PageKey::from_route(&AppRoute::Settings), PageKey::Settings);
        assert_eq!(PageKey::from_route(&AppRoute::Login), PageKey::Login);
    }

    #[test]
    fn playlist_routes_are_isolated_by_playlist_id() {
        assert_ne!(
            PageKey::from_route(&AppRoute::Playlist { id: 1 }),
            PageKey::from_route(&AppRoute::Playlist { id: 2 })
        );
    }

    #[test]
    fn search_routes_are_isolated_by_query_and_kind() {
        assert_ne!(
            PageKey::from_route(&AppRoute::SearchOverview {
                query: "yoasobi".into()
            }),
            PageKey::from_route(&AppRoute::SearchOverview {
                query: "aimer".into()
            })
        );
        assert_ne!(
            PageKey::from_route(&AppRoute::SearchCollection {
                query: "yoasobi".into(),
                kind: SearchCollectionKind::Albums,
            }),
            PageKey::from_route(&AppRoute::SearchCollection {
                query: "yoasobi".into(),
                kind: SearchCollectionKind::Tracks,
            })
        );
        assert_ne!(
            PageKey::from_route(&AppRoute::Search),
            PageKey::from_route(&AppRoute::SearchOverview {
                query: "yoasobi".into()
            })
        );
    }

    #[test]
    fn unknown_route_key_preserves_path() {
        assert_eq!(
            PageKey::from_route(&AppRoute::Unknown {
                path: "/mystery/path".into(),
            }),
            PageKey::Unknown("/mystery/path".into())
        );
    }
}
