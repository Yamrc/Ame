use std::sync::Arc;

use nekowg::{Image, SharedString};

#[derive(Debug, Clone)]
pub struct NeteaseAccountViewModel {
    pub auth_state: SharedString,
    pub account_summary: SharedString,
    pub qr_status: SharedString,
    pub qr_url: Option<SharedString>,
    pub qr_image: Option<Arc<Image>>,
    pub polling: bool,
}

#[derive(Debug, Clone)]
pub struct LastFmAccountViewModel {
    pub status: SharedString,
    pub account_summary: SharedString,
    pub queue_summary: SharedString,
    pub configured: bool,
    pub connected: bool,
    pub auth_loading: bool,
    pub error: Option<SharedString>,
}

#[derive(Debug, Clone)]
pub struct SettingsViewModel {
    pub close_behavior_label: SharedString,
    pub home_artist_language_label: SharedString,
    pub netease: NeteaseAccountViewModel,
    pub lastfm: LastFmAccountViewModel,
    pub app_error: Option<SharedString>,
}
