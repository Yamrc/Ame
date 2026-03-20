use crate::domain::library as library_actions;
use crate::page::state::{DataState, FreezablePageState};

#[derive(Debug, Clone, Default)]
pub struct HomePageState {
    pub recommend_playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
    pub recommend_artists: DataState<Vec<library_actions::ArtistItem>>,
    pub new_albums: DataState<Vec<library_actions::AlbumItem>>,
    pub toplists: DataState<Vec<library_actions::ToplistItem>>,
    pub daily_tracks: DataState<Vec<library_actions::DailyTrackItem>>,
    pub personal_fm: DataState<Option<library_actions::FmTrackItem>>,
}

impl HomePageState {
    pub fn clear(&mut self) {
        self.recommend_playlists.clear();
        self.recommend_artists.clear();
        self.new_albums.clear();
        self.toplists.clear();
        self.daily_tracks.clear();
        self.personal_fm.clear();
    }
}

impl FreezablePageState for HomePageState {
    fn release_for_freeze(&mut self) {
        self.clear();
    }
}
