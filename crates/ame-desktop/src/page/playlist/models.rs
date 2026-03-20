use crate::domain::library::{DailyTrackItem, PlaylistTrackItem};
use crate::domain::player;

pub type SessionLoadKey = (Option<i64>, bool);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlaylistTrackRow {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

impl From<PlaylistTrackRow> for player::QueueTrackInput {
    fn from(value: PlaylistTrackRow) -> Self {
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

impl From<PlaylistTrackItem> for PlaylistTrackRow {
    fn from(value: PlaylistTrackItem) -> Self {
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

impl From<DailyTrackItem> for PlaylistTrackRow {
    fn from(value: DailyTrackItem) -> Self {
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlaylistPage {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub tracks: Vec<PlaylistTrackRow>,
}
