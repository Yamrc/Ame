mod path;
mod segment;

use nekowg::SharedString;

use super::types::{AppRoute, SearchCollectionKind};
use path::normalize_path;
use segment::{decode_path_segment, encode_path_segment};

impl AppRoute {
    pub fn parse(path: &str) -> Self {
        let normalized = normalize_path(path);
        let trimmed = normalized.trim_matches('/');
        let segments = if trimmed.is_empty() {
            Vec::new()
        } else {
            trimmed.split('/').collect::<Vec<_>>()
        };

        match segments.as_slice() {
            [] => Self::Home,
            ["explore"] => Self::Explore,
            ["library"] => Self::Library,
            ["search"] => Self::Search,
            ["search", raw_query] => parse_search_overview(raw_query, &normalized),
            ["search", raw_query, kind] => parse_search_collection(raw_query, kind, &normalized),
            ["playlist", id] => parse_playlist(id, &normalized),
            ["daily", "songs"] => Self::DailyTracks,
            ["next"] => Self::Queue,
            ["settings"] => Self::Settings,
            ["login"] => Self::Login,
            _ => unknown_route(&normalized),
        }
    }

    pub fn to_path(&self) -> SharedString {
        match self {
            Self::Home => "/".into(),
            Self::Explore => "/explore".into(),
            Self::Library => "/library".into(),
            Self::Search => "/search".into(),
            Self::SearchOverview { query } => {
                format!("/search/{}", encode_path_segment(query)).into()
            }
            Self::SearchCollection { query, kind } => format!(
                "/search/{}/{}",
                encode_path_segment(query),
                kind.as_segment()
            )
            .into(),
            Self::Playlist { id } => format!("/playlist/{id}").into(),
            Self::DailyTracks => "/daily/songs".into(),
            Self::Queue => "/next".into(),
            Self::Settings => "/settings".into(),
            Self::Login => "/login".into(),
            Self::Unknown { path } => path.clone(),
        }
    }
}

fn parse_search_overview(raw_query: &str, normalized: &str) -> AppRoute {
    let Some(query) = decode_path_segment(raw_query) else {
        return unknown_route(normalized);
    };
    if query.trim().is_empty() {
        return unknown_route(normalized);
    }
    AppRoute::SearchOverview { query }
}

fn parse_search_collection(raw_query: &str, kind: &str, normalized: &str) -> AppRoute {
    let Some(query) = decode_path_segment(raw_query) else {
        return unknown_route(normalized);
    };
    if query.trim().is_empty() {
        return unknown_route(normalized);
    }
    let Some(kind) = SearchCollectionKind::from_segment(kind.trim()) else {
        return unknown_route(normalized);
    };
    AppRoute::SearchCollection { query, kind }
}

fn parse_playlist(id: &str, normalized: &str) -> AppRoute {
    match id.parse::<i64>() {
        Ok(id) if id > 0 => AppRoute::Playlist { id },
        _ => unknown_route(normalized),
    }
}

fn unknown_route(path: &str) -> AppRoute {
    AppRoute::Unknown {
        path: SharedString::from(path.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{AppRoute, SearchCollectionKind};

    #[test]
    fn parse_known_routes() {
        assert_eq!(AppRoute::parse("/"), AppRoute::Home);
        assert_eq!(AppRoute::parse("/explore"), AppRoute::Explore);
        assert_eq!(AppRoute::parse("/library"), AppRoute::Library);
        assert_eq!(
            AppRoute::parse("/search/yoasobi"),
            AppRoute::SearchOverview {
                query: "yoasobi".to_string()
            }
        );
        assert_eq!(
            AppRoute::parse("/search/yoasobi/albums"),
            AppRoute::SearchCollection {
                query: "yoasobi".to_string(),
                kind: SearchCollectionKind::Albums,
            }
        );
        assert_eq!(
            AppRoute::parse("/playlist/123"),
            AppRoute::Playlist { id: 123 }
        );
    }

    #[test]
    fn unknown_routes_are_explicit() {
        assert!(matches!(
            AppRoute::parse("/playlist/not-a-number"),
            AppRoute::Unknown { .. }
        ));
        assert!(matches!(
            AppRoute::parse("/foo/bar"),
            AppRoute::Unknown { .. }
        ));
    }

    #[test]
    fn route_to_path_roundtrip() {
        let route = AppRoute::SearchCollection {
            query: "yoasobi".to_string(),
            kind: SearchCollectionKind::Tracks,
        };
        assert_eq!(AppRoute::parse(route.to_path().as_ref()), route);
    }

    #[test]
    fn search_query_segment_is_percent_encoded() {
        let route = AppRoute::SearchOverview {
            query: "渚~君と目指した高み、願いが叶う場所/~".to_string(),
        };
        assert_eq!(
            route.to_path().as_ref(),
            "/search/%E6%B8%9A~%E5%90%9B%E3%81%A8%E7%9B%AE%E6%8C%87%E3%81%97%E3%81%9F%E9%AB%98%E3%81%BF%E3%80%81%E9%A1%98%E3%81%84%E3%81%8C%E5%8F%B6%E3%81%86%E5%A0%B4%E6%89%80%2F~"
        );
        assert_eq!(AppRoute::parse(route.to_path().as_ref()), route);
    }

    #[test]
    fn invalid_percent_encoding_is_unknown() {
        assert!(matches!(
            AppRoute::parse("/search/%E6%B8%9A%2"),
            AppRoute::Unknown { .. }
        ));
    }
}
