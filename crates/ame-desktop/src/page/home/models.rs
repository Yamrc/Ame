use crate::domain::library as library_actions;
use crate::page::state::DataState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HomeSessionKey {
    pub user_id: Option<i64>,
    pub has_user_token: bool,
    pub has_guest_token: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
pub struct HomePageSnapshot {
    pub loading: bool,
    pub error: Option<String>,
    pub daily_card: HomePlaylistCard,
    pub daily_first_track_id: Option<i64>,
    pub fm_card: HomePlaylistCard,
    pub fm_track: Option<library_actions::FmTrackItem>,
    pub playlists: Vec<HomePlaylistCard>,
    pub artists: Vec<HomeArtistCard>,
    pub albums: Vec<HomePlaylistCard>,
    pub toplists: Vec<HomePlaylistCard>,
}

impl HomePageSnapshot {
    pub fn from_states(
        recommend_playlists: &DataState<Vec<library_actions::LibraryPlaylistItem>>,
        recommend_artists: &DataState<Vec<library_actions::ArtistItem>>,
        new_albums: &DataState<Vec<library_actions::AlbumItem>>,
        toplists: &DataState<Vec<library_actions::ToplistItem>>,
        daily_tracks: &DataState<Vec<library_actions::DailyTrackItem>>,
        personal_fm: &DataState<Option<library_actions::FmTrackItem>>,
    ) -> Self {
        let loading = recommend_playlists.loading
            || recommend_artists.loading
            || new_albums.loading
            || toplists.loading
            || daily_tracks.loading
            || personal_fm.loading;
        let error = recommend_playlists
            .error
            .clone()
            .or(recommend_artists.error.clone())
            .or(new_albums.error.clone())
            .or(toplists.error.clone())
            .or(daily_tracks.error.clone())
            .or(personal_fm.error.clone());
        let daily_card = HomePlaylistCard {
            id: 0,
            name: "每日推荐".to_string(),
            subtitle: "根据你的口味更新".to_string(),
            cover_url: daily_tracks
                .data
                .first()
                .and_then(|track| track.cover_url.clone()),
        };
        let fm_card = personal_fm
            .data
            .as_ref()
            .map(|track| HomePlaylistCard {
                id: track.id,
                name: track.name.clone(),
                subtitle: track.artists.clone(),
                cover_url: track.cover_url.clone(),
            })
            .unwrap_or(HomePlaylistCard {
                id: 0,
                name: "私人 FM".to_string(),
                subtitle: "连续播放你可能喜欢的音乐".to_string(),
                cover_url: None,
            });

        Self {
            loading,
            error,
            daily_card,
            daily_first_track_id: daily_tracks.data.first().map(|track| track.id),
            fm_card,
            fm_track: personal_fm.data.clone(),
            playlists: recommend_playlists
                .data
                .iter()
                .take(10)
                .map(|item| HomePlaylistCard {
                    id: item.id,
                    name: item.name.clone(),
                    subtitle: item.creator_name.clone(),
                    cover_url: item.cover_url.clone(),
                })
                .collect(),
            artists: recommend_artists
                .data
                .iter()
                .take(6)
                .map(|artist| HomeArtistCard {
                    name: artist.name.clone(),
                    cover_url: artist.cover_url.clone(),
                })
                .collect(),
            albums: new_albums
                .data
                .iter()
                .take(10)
                .map(|album| HomePlaylistCard {
                    id: album.id,
                    name: album.name.clone(),
                    subtitle: album.artist_name.clone(),
                    cover_url: album.cover_url.clone(),
                })
                .collect(),
            toplists: toplists
                .data
                .iter()
                .take(10)
                .map(|list| HomePlaylistCard {
                    id: list.id,
                    name: list.name.clone(),
                    subtitle: list.update_frequency.clone(),
                    cover_url: list.cover_url.clone(),
                })
                .collect(),
        }
    }
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
