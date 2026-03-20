use std::sync::Arc;

use nekowg::Image;

#[derive(Debug, Clone, Default)]
pub struct LoginViewModel {
    pub auth_state: String,
    pub account_summary: Option<String>,
    pub qr_status: Option<String>,
    pub qr_url: Option<String>,
    pub qr_image: Option<Arc<Image>>,
    pub polling: bool,
    pub error: Option<String>,
}
