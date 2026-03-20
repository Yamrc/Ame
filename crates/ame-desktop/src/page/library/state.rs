use crate::domain::library::{LibraryPlaylistItem, PlaylistTrackItem};
use crate::page::library::models::LibraryTab;
use crate::page::state::{DataState, FreezablePageState};

#[derive(Debug, Clone)]
pub struct LibraryPageState {
    pub playlists: DataState<Vec<LibraryPlaylistItem>>,
    pub liked_tracks: DataState<Vec<PlaylistTrackItem>>,
    pub liked_lyric_lines: Vec<String>,
    pub tab: LibraryTab,
}

impl Default for LibraryPageState {
    fn default() -> Self {
        Self {
            playlists: DataState::default(),
            liked_tracks: DataState::default(),
            liked_lyric_lines: Vec::new(),
            tab: LibraryTab::Created,
        }
    }
}

impl LibraryPageState {
    pub fn clear(&mut self) {
        self.playlists.clear();
        self.liked_tracks.clear();
        self.liked_lyric_lines.clear();
    }
}

impl FreezablePageState for LibraryPageState {
    fn release_for_freeze(&mut self) {
        self.clear();
    }
}
