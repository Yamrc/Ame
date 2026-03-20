use crate::domain::library as library_actions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeSessionKey {
    pub user_id: Option<i64>,
    pub has_user_token: bool,
    pub has_guest_token: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HomePlaylistCard {
    pub id: i64,
    pub name: String,
    pub subtitle: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeArtistCard {
    pub name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HomeLoadResult {
    pub recommend_playlists: Vec<library_actions::LibraryPlaylistItem>,
    pub recommend_artists: Vec<library_actions::ArtistItem>,
    pub new_albums: Vec<library_actions::AlbumItem>,
    pub toplists: Vec<library_actions::ToplistItem>,
    pub daily_tracks: Vec<library_actions::DailyTrackItem>,
    pub personal_fm: Option<library_actions::FmTrackItem>,
    pub fetched_at_ms: u64,
}
