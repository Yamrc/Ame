use std::{collections::HashMap, sync::Arc, time::Instant};

use crate::action::library_actions;
use crate::view::{library, playlist, search};
use nekowg::Image;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSource {
    Guest,
    User,
}

#[derive(Debug, Clone)]
pub struct DataState<T> {
    pub data: T,
    pub loading: bool,
    pub error: Option<String>,
    pub fetched_at_ms: Option<u64>,
    pub source: DataSource,
}

impl<T: Default> Default for DataState<T> {
    fn default() -> Self {
        Self {
            data: T::default(),
            loading: false,
            error: None,
            fetched_at_ms: None,
            source: DataSource::Guest,
        }
    }
}

impl<T: Default> DataState<T> {
    pub fn begin(&mut self, source: DataSource) {
        self.loading = true;
        self.error = None;
        self.source = source;
    }

    pub fn succeed(&mut self, data: T, fetched_at_ms: Option<u64>) {
        self.data = data;
        self.loading = false;
        self.error = None;
        self.fetched_at_ms = fetched_at_ms;
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.loading = false;
        self.error = Some(error.into());
    }

    pub fn clear(&mut self) {
        self.data = T::default();
        self.loading = false;
        self.error = None;
        self.fetched_at_ms = None;
    }
}

#[derive(Debug, Clone, Default)]
pub struct HomePageState {
    pub recommend_playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
    pub recommend_artists: DataState<Vec<library_actions::ArtistItem>>,
    pub new_albums: DataState<Vec<library_actions::AlbumItem>>,
    pub toplists: DataState<Vec<library_actions::ToplistItem>>,
    pub daily_tracks: DataState<Vec<library_actions::DailyTrackItem>>,
    pub personal_fm: DataState<Option<library_actions::FmTrackItem>>,
}

#[derive(Debug, Clone, Default)]
pub struct DiscoverPageState {
    pub playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
}

#[derive(Debug, Clone)]
pub struct LibraryPageState {
    pub playlists: DataState<Vec<library_actions::LibraryPlaylistItem>>,
    pub liked_tracks: DataState<Vec<library_actions::PlaylistTrackItem>>,
    pub liked_lyric_lines: Vec<String>,
    pub tab: library::LibraryTab,
}

impl Default for LibraryPageState {
    fn default() -> Self {
        Self {
            playlists: DataState::default(),
            liked_tracks: DataState::default(),
            liked_lyric_lines: Vec::new(),
            tab: library::LibraryTab::Created,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchCollectionState<T> {
    pub keyword: String,
    pub items: DataState<Vec<T>>,
    pub has_more: bool,
}

impl<T> Default for SearchCollectionState<T> {
    fn default() -> Self {
        Self {
            keyword: String::new(),
            items: DataState::default(),
            has_more: true,
        }
    }
}

impl<T> SearchCollectionState<T> {
    pub fn clear(&mut self) {
        self.keyword.clear();
        self.items.clear();
        self.has_more = true;
    }
}

#[derive(Debug, Clone, Default)]
pub struct SearchPageState {
    pub overview_keyword: String,
    pub overview: DataState<search::SearchOverview>,
    pub artists: SearchCollectionState<search::SearchArtist>,
    pub albums: SearchCollectionState<search::SearchAlbum>,
    pub tracks: SearchCollectionState<search::SearchSong>,
    pub playlists: SearchCollectionState<search::SearchPlaylist>,
}

#[derive(Debug, Clone)]
pub struct PlaylistPageState {
    pub pages: DataState<HashMap<i64, playlist::PlaylistPage>>,
}

impl Default for PlaylistPageState {
    fn default() -> Self {
        Self {
            pages: DataState {
                data: HashMap::new(),
                ..DataState::default()
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LoginPageState {
    pub qr_key: Option<String>,
    pub qr_url: Option<String>,
    pub qr_image: Option<Arc<Image>>,
    pub qr_status: Option<String>,
    pub qr_polling: bool,
    pub qr_poll_started_at: Option<Instant>,
    pub qr_last_polled_at: Option<Instant>,
}
