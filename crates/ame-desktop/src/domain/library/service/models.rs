use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryPlaylistItem {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
    #[serde(default)]
    pub creator_id: Option<i64>,
    #[serde(default)]
    pub subscribed: bool,
    #[serde(default)]
    pub special_type: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistTrackItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaylistDetailData {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub tracks: Vec<PlaylistTrackItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FmTrackItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyTrackItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtistItem {
    pub id: i64,
    pub name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumItem {
    pub id: i64,
    pub name: String,
    pub artist_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToplistItem {
    pub id: i64,
    pub name: String,
    pub update_frequency: String,
    pub cover_url: Option<String>,
}
