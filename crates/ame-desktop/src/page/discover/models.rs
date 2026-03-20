use crate::domain::library::LibraryPlaylistItem;
use crate::page::state::DataState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverPlaylistCard {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiscoverPageSnapshot {
    pub loading: bool,
    pub error: Option<String>,
    pub playlists: Vec<DiscoverPlaylistCard>,
}

#[derive(Debug, Clone)]
pub struct DiscoverLoadResult {
    pub playlists: Vec<LibraryPlaylistItem>,
    pub fetched_at_ms: u64,
}

impl DiscoverPageSnapshot {
    pub fn from_state(state: &DataState<Vec<LibraryPlaylistItem>>) -> Self {
        Self {
            loading: state.loading,
            error: state.error.clone(),
            playlists: state
                .data
                .iter()
                .take(12)
                .map(|item| DiscoverPlaylistCard {
                    id: item.id,
                    name: item.name.clone(),
                    track_count: item.track_count,
                    creator_name: item.creator_name.clone(),
                    cover_url: item.cover_url.clone(),
                })
                .collect(),
        }
    }
}
