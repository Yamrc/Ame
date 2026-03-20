use crate::domain::library::DailyTrackItem;
use crate::page::state::{DataState, FreezablePageState};

#[derive(Debug, Clone, Default)]
pub struct DailyTracksPageState {
    pub tracks: DataState<Vec<DailyTrackItem>>,
}

impl DailyTracksPageState {}

impl FreezablePageState for DailyTracksPageState {
    fn release_for_freeze(&mut self) {
        self.tracks.clear();
    }
}
