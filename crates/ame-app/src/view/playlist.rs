#[derive(Debug, Clone, Default)]
pub struct PlaylistViewModel {
    pub playlist_id: i64,
    pub track_ids: Vec<i64>,
}
