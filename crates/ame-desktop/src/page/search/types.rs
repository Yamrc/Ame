use crate::app::route::{AppRoute, SearchCollectionKind};
use crate::domain::player;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchRouteType {
    Artists,
    Albums,
    Tracks,
    Playlists,
}

impl SearchRouteType {
    pub const fn from_kind(kind: SearchCollectionKind) -> Self {
        match kind {
            SearchCollectionKind::Artists => Self::Artists,
            SearchCollectionKind::Albums => Self::Albums,
            SearchCollectionKind::Tracks => Self::Tracks,
            SearchCollectionKind::Playlists => Self::Playlists,
        }
    }

    pub const fn as_kind(self) -> SearchCollectionKind {
        match self {
            Self::Artists => SearchCollectionKind::Artists,
            Self::Albums => SearchCollectionKind::Albums,
            Self::Tracks => SearchCollectionKind::Tracks,
            Self::Playlists => SearchCollectionKind::Playlists,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Artists => "艺人",
            Self::Albums => "专辑",
            Self::Tracks => "歌曲",
            Self::Playlists => "歌单",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SearchPageRoute {
    pub keyword: String,
    pub route_type: Option<SearchRouteType>,
}

impl SearchPageRoute {
    pub fn from_app_route(route: &AppRoute) -> Option<Self> {
        match route {
            AppRoute::Search => Some(Self::default()),
            AppRoute::SearchOverview { query } => Some(Self {
                keyword: query.clone(),
                route_type: None,
            }),
            AppRoute::SearchCollection { query, kind } => Some(Self {
                keyword: query.clone(),
                route_type: Some(SearchRouteType::from_kind(*kind)),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSong {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

impl From<SearchSong> for player::QueueTrackInput {
    fn from(value: SearchSong) -> Self {
        Self {
            id: value.id,
            name: value.name,
            alias: value.alias,
            artists: value.artists,
            album: value.album,
            duration_ms: value.duration_ms,
            cover_url: value.cover_url,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchArtist {
    pub id: i64,
    pub name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAlbum {
    pub id: i64,
    pub name: String,
    pub artist_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPlaylist {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchOverview {
    pub artists: Vec<SearchArtist>,
    pub albums: Vec<SearchAlbum>,
    pub tracks: Vec<SearchSong>,
    pub playlists: Vec<SearchPlaylist>,
}

impl SearchOverview {
    pub fn has_result(&self) -> bool {
        !self.artists.is_empty()
            || !self.albums.is_empty()
            || !self.tracks.is_empty()
            || !self.playlists.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct SearchPageSlice<T> {
    pub items: Vec<T>,
    pub has_more: bool,
}

#[derive(Debug, Clone)]
pub enum SearchTypePayload {
    Artists(SearchPageSlice<SearchArtist>),
    Albums(SearchPageSlice<SearchAlbum>),
    Tracks(SearchPageSlice<SearchSong>),
    Playlists(SearchPageSlice<SearchPlaylist>),
}
