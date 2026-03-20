use std::rc::Rc;

use nekowg::{AnyElement, App};

use crate::component::track_item::{self, TrackItemActions, TrackItemProps};
use crate::page::playlist::models::PlaylistTrackRow;

pub(crate) fn track_row(
    state_id: impl Into<nekowg::SharedString>,
    item: PlaylistTrackRow,
    is_playing: bool,
    on_play: impl Fn(&mut App) + 'static,
    on_enqueue: impl Fn(&mut App) + 'static,
) -> AnyElement {
    track_item::render(
        TrackItemProps {
            id: item.id,
            state_id: state_id.into(),
            title: item.name,
            alias: item.alias,
            artists: item.artists,
            album: item.album,
            duration_ms: item.duration_ms,
            cover_url: item.cover_url,
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
