use nekowg::SharedString;

use crate::domain::library::LibraryPlaylistItem;
use crate::page::state::DataState;

#[derive(Debug, Clone)]
pub struct LibraryLoadResult {
    pub playlists: Vec<LibraryPlaylistItem>,
    pub liked_tracks: Vec<crate::domain::library::PlaylistTrackItem>,
    pub liked_lyric_lines: Vec<String>,
    pub fetched_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPlaylistCard {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Created,
    Collected,
    Followed,
}

#[derive(Clone)]
pub struct LibraryPageSnapshot {
    pub title: SharedString,
    pub user_avatar: Option<String>,
    pub loading: bool,
    pub error: Option<SharedString>,
    pub liked_playlist: Option<LibraryPlaylistCard>,
    pub liked_lyric_lines: Vec<String>,
    pub liked_tracks: Vec<crate::domain::library::PlaylistTrackItem>,
    pub active_tab: LibraryTab,
    pub created_playlists: Vec<LibraryPlaylistCard>,
    pub collected_playlists: Vec<LibraryPlaylistCard>,
    pub followed_playlists: Vec<LibraryPlaylistCard>,
}

impl LibraryPageSnapshot {
    pub fn from_state(
        playlists_state: &DataState<Vec<LibraryPlaylistItem>>,
        liked_tracks_state: &DataState<Vec<crate::domain::library::PlaylistTrackItem>>,
        liked_lyric_lines: &[String],
        active_tab: LibraryTab,
        auth_account_summary: Option<&str>,
        auth_user_name: Option<&str>,
        auth_user_avatar: Option<&str>,
    ) -> Self {
        let liked_playlist = playlists_state
            .data
            .iter()
            .find(|item| item.special_type == 5)
            .map(Self::map_playlist_card);
        let created_playlists = playlists_state
            .data
            .iter()
            .filter(|item| !item.subscribed && item.special_type != 5)
            .map(Self::map_playlist_card)
            .collect();
        let collected_playlists = playlists_state
            .data
            .iter()
            .filter(|item| item.subscribed)
            .map(Self::map_playlist_card)
            .collect();
        let title = auth_user_name
            .filter(|name| !name.trim().is_empty())
            .map(|name| format!("{name} 的音乐库"))
            .or_else(|| {
                auth_account_summary
                    .filter(|summary| !summary.trim().is_empty())
                    .map(|summary| format!("{summary} 的音乐库"))
            })
            .unwrap_or_else(|| "我的音乐库".to_string());

        Self {
            title: title.into(),
            user_avatar: auth_user_avatar.map(ToOwned::to_owned),
            loading: playlists_state.loading,
            error: playlists_state.error.clone().map(Into::into),
            liked_playlist,
            liked_lyric_lines: liked_lyric_lines.to_vec(),
            liked_tracks: liked_tracks_state.data.clone(),
            active_tab,
            created_playlists,
            collected_playlists,
            followed_playlists: Vec::new(),
        }
    }

    fn map_playlist_card(item: &LibraryPlaylistItem) -> LibraryPlaylistCard {
        LibraryPlaylistCard {
            id: item.id,
            name: item.name.clone(),
            track_count: item.track_count,
            creator_name: item.creator_name.clone(),
            cover_url: item.cover_url.clone(),
        }
    }
}
