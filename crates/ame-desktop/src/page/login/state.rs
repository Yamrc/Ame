use std::sync::Arc;
use std::time::Instant;

use nekowg::Image;

#[derive(Debug, Clone, Default)]
pub struct LoginPageState {
    pub qr_key: Option<String>,
    pub qr_url: Option<String>,
    pub qr_image: Option<Arc<Image>>,
    pub qr_status: Option<String>,
    pub qr_polling: bool,
    pub qr_poll_started_at: Option<Instant>,
    pub qr_last_polled_at: Option<Instant>,
}
