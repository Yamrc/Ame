use crate::domain::library::DailyTrackItem;
use crate::page::playlist::PlaylistTrackRow;
use crate::page::state::DataState;

#[derive(Debug, Clone)]
pub struct DailyTracksPageSnapshot {
    pub loading: bool,
    pub error: Option<String>,
    pub tracks: Vec<PlaylistTrackRow>,
    pub first_track_id: Option<i64>,
    pub current_playing_track_id: Option<i64>,
}

impl DailyTracksPageSnapshot {
    pub fn from_state(
        state: &DataState<Vec<DailyTrackItem>>,
        current_playing_track_id: Option<i64>,
    ) -> Self {
        Self {
            loading: state.loading,
            error: state.error.clone(),
            tracks: state
                .data
                .iter()
                .cloned()
                .map(PlaylistTrackRow::from)
                .collect(),
            first_track_id: state.data.first().map(|track| track.id),
            current_playing_track_id,
        }
    }
}
