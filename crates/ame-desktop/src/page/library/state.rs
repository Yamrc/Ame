use crate::domain::library::{LibraryPlaylistItem, PlaylistTrackItem};
use crate::page::library::models::LibraryTab;
use crate::page::state::{DataState, FreezablePageState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryPageFrozenState {
    pub tab: LibraryTab,
}

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

impl LibraryPageState {
    pub fn frozen_state(&self) -> LibraryPageFrozenState {
        LibraryPageFrozenState { tab: self.tab }
    }

    pub fn restore_frozen_state(&mut self, frozen: LibraryPageFrozenState) {
        self.tab = frozen.tab;
    }
}

#[cfg(test)]
mod tests {
    use super::{LibraryPageFrozenState, LibraryPageState};
    use crate::page::library::models::LibraryTab;
    use crate::page::state::FreezablePageState;

    #[test]
    fn release_for_freeze_preserves_tab() {
        let mut state = LibraryPageState {
            tab: LibraryTab::Collected,
            ..Default::default()
        };
        state.release_for_freeze();
        assert_eq!(state.tab, LibraryTab::Collected);
    }

    #[test]
    fn restore_frozen_state_restores_tab() {
        let mut state = LibraryPageState::default();
        state.restore_frozen_state(LibraryPageFrozenState {
            tab: LibraryTab::Followed,
        });
        assert_eq!(state.tab, LibraryTab::Followed);
    }
}
