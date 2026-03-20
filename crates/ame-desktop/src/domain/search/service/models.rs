#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPage<T> {
    pub items: Vec<T>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSongItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artists: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchArtistItem {
    pub id: i64,
    pub name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchAlbumItem {
    pub id: i64,
    pub name: String,
    pub artist_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchPlaylistItem {
    pub id: i64,
    pub name: String,
    pub creator_name: String,
    pub track_count: u32,
    pub cover_url: Option<String>,
}
