use crate::domain::library::LibraryPlaylistItem;
use crate::page::state::{DataState, FreezablePageState};

#[derive(Debug, Clone, Default)]
pub struct DiscoverPageState {
    pub playlists: DataState<Vec<LibraryPlaylistItem>>,
}

impl DiscoverPageState {
    pub fn clear(&mut self) {
        self.playlists.clear();
    }
}

impl FreezablePageState for DiscoverPageState {
    fn release_for_freeze(&mut self) {
        self.clear();
    }
}
