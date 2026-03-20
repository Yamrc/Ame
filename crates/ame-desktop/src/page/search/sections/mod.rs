mod collection;
mod overview;

use std::rc::Rc;
use std::sync::Arc;

use nekowg::{AnyElement, App, SharedString, px};

use crate::component::{
    cover_card::{self, ArtistCoverCardProps, CoverCardActions},
    playlist_card::{self, PlaylistCardActions, PlaylistCardProps},
    short_track_item::{self, ShortTrackItemActions, ShortTrackItemProps},
    track_item::{self, TrackItemActions, TrackItemProps},
};

use super::state::{SearchCollectionState, SearchPageState};
use super::types::{SearchOverview, SearchRouteType, SearchSong};

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
pub(crate) type NavigateHandler = Rc<dyn Fn(&mut App)>;
pub(crate) type PlaylistOpenHandler = Rc<dyn Fn(i64, &mut App)>;
pub(crate) type SearchTypeNavigateHandler = Rc<dyn Fn(SearchRouteType, &mut App)>;
type CardOpenHandler = Rc<dyn Fn(&mut App)>;

pub(crate) use collection::render_type_page;
pub(crate) use overview::render_overview_sections;

fn render_track_row(
    state_id: impl Into<SharedString>,
    song: SearchSong,
    is_playing: bool,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
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
        },
        TrackItemActions {
            on_play: Some(Rc::new(on_play)),
            on_enqueue: Some(Rc::new(on_enqueue)),
            ..TrackItemActions::default()
        },
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

fn render_artist_card(name: impl Into<String>, cover_url: Option<String>) -> AnyElement {
    cover_card::render_artist_card(
        ArtistCoverCardProps {
            name: name.into(),
            cover_url,
        },
        CoverCardActions::default(),
    )
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
