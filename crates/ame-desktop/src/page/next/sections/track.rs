use nekowg::AnyElement;

use crate::component::track_item::{
    self, TrackItemActions, TrackItemFavoriteState, TrackItemProps,
};
use crate::domain::player::QueueItem;

use super::QueueActionHandler;

pub(super) fn queue_track_row(
    state_id: impl Into<nekowg::SharedString>,
    item: QueueItem,
    is_playing: bool,
    favorite: TrackItemFavoriteState,
    on_play: Option<QueueActionHandler>,
    on_toggle_favorite: QueueActionHandler,
    on_remove: Option<QueueActionHandler>,
) -> AnyElement {
    track_item::render(
        TrackItemProps {
            id: item.id,
            state_id: state_id.into(),
            title: item.name,
            alias: item.alias,
            artists: item.artist,
            album: item.album.clone(),
            duration_ms: item.duration_ms,
            cover_url: item.cover_url,
            show_cover: true,
            is_playing,
            favorite,
        },
        TrackItemActions {
            on_play,
            on_toggle_favorite: Some(on_toggle_favorite),
            on_remove,
            ..TrackItemActions::default()
        },
    )
}
