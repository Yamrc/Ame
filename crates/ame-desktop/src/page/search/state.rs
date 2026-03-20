use std::time::{SystemTime, UNIX_EPOCH};

use crate::page::state::{DataSource, DataState, FreezablePageState};

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
    pub overview: DataState<super::types::SearchOverview>,
    pub artists: SearchCollectionState<super::types::SearchArtist>,
    pub albums: SearchCollectionState<super::types::SearchAlbum>,
    pub tracks: SearchCollectionState<super::types::SearchSong>,
    pub playlists: SearchCollectionState<super::types::SearchPlaylist>,
}

impl SearchPageState {
    pub fn clear_all(&mut self) {
        self.overview_keyword.clear();
        self.overview.clear();
        self.artists.clear();
        self.albums.clear();
        self.tracks.clear();
        self.playlists.clear();
    }
}

impl FreezablePageState for SearchPageState {
    fn release_for_freeze(&mut self) {
        self.clear_all();
    }
}

pub fn should_skip_collection_load<T>(
    state: &SearchCollectionState<T>,
    keyword: &str,
    append: bool,
) -> bool {
    if append {
        state.items.loading || !state.has_more || state.keyword != keyword
    } else {
        state.items.loading || (state.keyword == keyword && state.items.fetched_at_ms.is_some())
    }
}

pub fn prepare_collection_load<T>(
    state: &mut SearchCollectionState<T>,
    keyword: String,
    append: bool,
    source: DataSource,
) {
    if !append {
        state.keyword = keyword;
        state.items.data.clear();
        state.items.fetched_at_ms = None;
        state.has_more = true;
    }
    state.items.begin(source);
}

pub fn apply_collection_result<T>(
    state: &mut SearchCollectionState<T>,
    keyword: String,
    page: super::types::SearchPageSlice<T>,
    append: bool,
) {
    state.keyword = keyword;
    state.has_more = page.has_more;
    if append {
        state.items.data.extend(page.items);
        state.items.loading = false;
        state.items.error = None;
        state.items.fetched_at_ms = Some(now_millis());
    } else {
        state.items.succeed(page.items, Some(now_millis()));
    }
}

pub fn apply_collection_error<T>(
    state: &mut SearchCollectionState<T>,
    keyword: String,
    error: String,
    append: bool,
) {
    state.keyword = keyword;
    if !append {
        state.items.clear();
    }
    state.items.fail(error);
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
