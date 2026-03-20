use crate::page::playlist::models::PlaylistPage;
use crate::page::state::{DataState, FreezablePageState};

#[derive(Debug, Clone, Default)]
pub struct PlaylistPageState {
    pub page: DataState<Option<PlaylistPage>>,
}

impl PlaylistPageState {}

impl FreezablePageState for PlaylistPageState {
    fn release_for_freeze(&mut self) {
        self.page.clear();
    }
}
