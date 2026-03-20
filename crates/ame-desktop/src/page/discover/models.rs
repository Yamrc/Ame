use crate::domain::library::LibraryPlaylistItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverPlaylistCard {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiscoverLoadResult {
    pub playlists: Vec<LibraryPlaylistItem>,
    pub fetched_at_ms: u64,
}
