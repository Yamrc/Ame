use nekowg::SharedString;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchCollectionKind {
    Artists,
    Albums,
    Tracks,
    Playlists,
}

impl SearchCollectionKind {
    pub fn from_segment(segment: &str) -> Option<Self> {
        match segment.trim() {
            "artists" => Some(Self::Artists),
            "albums" => Some(Self::Albums),
            "tracks" => Some(Self::Tracks),
            "playlists" => Some(Self::Playlists),
            _ => None,
        }
    }

    pub const fn as_segment(self) -> &'static str {
        match self {
            Self::Artists => "artists",
            Self::Albums => "albums",
            Self::Tracks => "tracks",
            Self::Playlists => "playlists",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AppRoute {
    #[default]
    Home,
    Explore,
    Library,
    Search,
    SearchOverview {
        query: String,
    },
    SearchCollection {
        query: String,
        kind: SearchCollectionKind,
    },
    Playlist {
        id: i64,
    },
    DailyTracks,
    Queue,
    Settings,
    Login,
    Unknown {
        path: SharedString,
    },
}
