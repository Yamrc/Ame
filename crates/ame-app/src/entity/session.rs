#[derive(Debug, Clone, Default)]
pub struct SessionEntity {
    pub cookie_loaded: bool,
    pub logged_in: bool,
    pub user_id: Option<i64>,
    pub nickname: Option<String>,
}
