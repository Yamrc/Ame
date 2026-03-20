use crate::domain::library::{DailyTrackItem, PlaylistTrackItem};
use crate::domain::player;
use crate::page::state::DataState;

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

#[derive(Debug, Clone)]
pub struct PlaylistPageSnapshot {
    pub playlist_id: i64,
    pub loading: bool,
    pub error: Option<String>,
    pub playlist: Option<PlaylistPage>,
    pub current_playing_track_id: Option<i64>,
}

impl PlaylistPageSnapshot {
    pub fn from_state(
        playlist_id: i64,
        state: &DataState<Option<PlaylistPage>>,
        current_playing_track_id: Option<i64>,
    ) -> Self {
        Self {
            playlist_id,
            loading: state.loading,
            error: state.error.clone(),
            playlist: state.data.clone(),
            current_playing_track_id,
        }
    }
}
