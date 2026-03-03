#[derive(Debug, Clone, Default)]
pub struct SearchViewModel {
    pub keyword: String,
    pub song_ids: Vec<i64>,
}
