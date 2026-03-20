use crate::domain::library;

#[derive(Debug, Clone)]
pub struct QueueTrackInput {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

impl From<library::PlaylistTrackItem> for QueueTrackInput {
    fn from(value: library::PlaylistTrackItem) -> Self {
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

impl From<library::DailyTrackItem> for QueueTrackInput {
    fn from(value: library::DailyTrackItem) -> Self {
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

impl From<library::FmTrackItem> for QueueTrackInput {
    fn from(value: library::FmTrackItem) -> Self {
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
