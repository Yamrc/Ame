use crate::domain::library::LibraryPlaylistItem;

#[derive(Debug, Clone)]
pub struct LibraryLoadResult {
    pub playlists: Vec<LibraryPlaylistItem>,
    pub liked_tracks: Vec<crate::domain::library::PlaylistTrackItem>,
    pub liked_lyric_lines: Vec<String>,
    pub fetched_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPlaylistCard {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Created,
    Collected,
    Followed,
}
