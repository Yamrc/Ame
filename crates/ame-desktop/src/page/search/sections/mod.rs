mod collection;
mod overview;

use std::rc::Rc;
use std::sync::Arc;

use nekowg::{AnyElement, App, SharedString, px};

use crate::component::{
    cover_card::{self, ArtistCoverCardProps, CoverCardActions},
    playlist_card::{self, PlaylistCardActions, PlaylistCardProps},
    short_track_item::{self, ShortTrackItemActions, ShortTrackItemProps},
    track_item::TrackItemFavoriteState,
    track_item::{self, TrackItemActions, TrackItemProps},
};
use crate::domain::favorites::FavoritesState;

use super::state::{SearchCollectionState, SearchPageState};
use super::types::{
    SearchAlbum, SearchArtist, SearchOverview, SearchPlaylist, SearchRouteType, SearchSong,
};

const PLAYLIST_GRID_COLUMNS: usize = 6;
const SHORT_TRACK_COLUMNS: usize = 4;
const SHORT_TRACK_HEIGHT: f32 = 48.0;
const SHORT_TRACK_GRID_GAP: f32 = 12.0;
const SEARCH_TYPE_CARD_COLUMNS: usize = 5;
const OVERVIEW_ARTIST_PLACEHOLDER_HEIGHT: f32 = 180.0;
const OVERVIEW_CARD_PLACEHOLDER_HEIGHT: f32 = 166.0;
const OVERVIEW_TRACK_PLACEHOLDER_HEIGHT: f32 = SHORT_TRACK_HEIGHT + 8.0;

pub(crate) type PlaySongHandler = Arc<dyn Fn(SearchSong, &mut App)>;
pub(crate) type EnqueueSongHandler = Arc<dyn Fn(SearchSong, &mut App)>;
pub(crate) type FavoriteSongHandler = Rc<dyn Fn(i64, &mut App)>;
pub(crate) type NavigateHandler = Rc<dyn Fn(&mut App)>;
pub(crate) type PlaylistOpenHandler = Rc<dyn Fn(i64, &mut App)>;
pub(crate) type SearchTypeNavigateHandler = Rc<dyn Fn(SearchRouteType, &mut App)>;
type CardOpenHandler = Rc<dyn Fn(&mut App)>;

#[derive(Clone)]
pub(crate) struct SearchFavoriteState {
    pub favorites: FavoritesState,
    pub ready: bool,
}

#[derive(Clone)]
pub(crate) struct SearchTypeRenderActions {
    pub on_play_song: PlaySongHandler,
    pub on_enqueue_song: EnqueueSongHandler,
    pub on_toggle_favorite: FavoriteSongHandler,
    pub on_open_playlist: PlaylistOpenHandler,
    pub on_load_more: NavigateHandler,
}

pub(crate) use collection::render_type_page;
pub(crate) use overview::render_overview_sections;

fn render_track_row(
    state_id: impl Into<SharedString>,
    song: SearchSong,
    is_playing: bool,
    favorite: TrackItemFavoriteState,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
    on_toggle_favorite: impl Fn(&mut App) + 'static,
) -> AnyElement {
    track_item::render(
        TrackItemProps {
            id: song.id,
            state_id: state_id.into(),
            title: song.name,
            alias: song.alias,
            artists: song.artists,
            album: song.album,
            duration_ms: song.duration_ms,
            cover_url: song.cover_url,
            show_cover: true,
            is_playing,
            favorite,
        },
        TrackItemActions {
            on_play: Some(Rc::new(on_play)),
            on_enqueue: Some(Rc::new(on_enqueue)),
            on_toggle_favorite: Some(Rc::new(on_toggle_favorite)),
            ..TrackItemActions::default()
        },
    )
}

fn render_track_row_ref(
    state_id: impl Into<SharedString>,
    song: &SearchSong,
    is_playing: bool,
    favorite: TrackItemFavoriteState,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
    on_toggle_favorite: impl Fn(&mut App) + 'static,
) -> AnyElement {
    render_track_row(
        state_id,
        song.clone(),
        is_playing,
        favorite,
        on_play,
        on_enqueue,
        on_toggle_favorite,
    )
}

fn render_short_track_item(
    state_id: impl Into<SharedString>,
    song: SearchSong,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
) -> AnyElement {
    short_track_item::render(
        ShortTrackItemProps {
            id: song.id,
            state_id: state_id.into(),
            title: song.name,
            subtitle: song.artists,
            cover_url: song.cover_url,
            height: px(SHORT_TRACK_HEIGHT),
        },
        ShortTrackItemActions {
            on_play: Some(Rc::new(on_play)),
            on_enqueue: Some(Rc::new(on_enqueue)),
        },
    )
}

fn render_short_track_item_ref(
    state_id: impl Into<SharedString>,
    song: &SearchSong,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
) -> AnyElement {
    render_short_track_item(state_id, song.clone(), on_play, on_enqueue)
}

fn render_artist_card(name: impl Into<String>, cover_url: Option<String>) -> AnyElement {
    cover_card::render_artist_card(
        ArtistCoverCardProps {
            name: name.into(),
            cover_url,
        },
        CoverCardActions::default(),
    )
}

fn render_artist_card_ref(artist: &SearchArtist) -> AnyElement {
    render_artist_card(artist.name.clone(), artist.cover_url.clone())
}

fn render_playlist_card(
    name: impl Into<String>,
    subtitle: impl Into<String>,
    cover_url: Option<String>,
    on_open: Option<CardOpenHandler>,
) -> AnyElement {
    playlist_card::render(
        PlaylistCardProps::standard(name, subtitle, cover_url),
        PlaylistCardActions { on_open },
    )
}

fn render_album_card_ref(album: &SearchAlbum) -> AnyElement {
    render_playlist_card(
        album.name.clone(),
        album.artist_name.clone(),
        album.cover_url.clone(),
        None,
    )
}

fn render_playlist_card_ref(
    playlist: &SearchPlaylist,
    on_open: Option<CardOpenHandler>,
) -> AnyElement {
    render_playlist_card(
        playlist.name.clone(),
        playlist.creator_name.clone(),
        playlist.cover_url.clone(),
        on_open,
    )
}
