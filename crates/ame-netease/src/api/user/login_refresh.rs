use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoginRefreshResponse {
    pub code: i64,
    #[serde(default)]
    pub cookie: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub msg: Option<String>,
}

pub struct LoginRefreshRequest;

impl ApiRequest for LoginRefreshRequest {
    type Response = LoginRefreshResponse;

    fn endpoint(&self) -> &'static str {
        "/api/login/token/refresh"
    }

    fn payload(&self) -> Value {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::LoginRefreshRequest;

    #[test]
    fn payload_matches_api_enhanced_shape() {
        let req = LoginRefreshRequest;
        assert_eq!(req.endpoint(), "/api/login/token/refresh");
        assert_eq!(req.payload(), serde_json::json!({}));
    }

    #[tokio::test]
    async fn live_login_refresh_request() {
        let client = crate::NeteaseClient::with_cookie(
            "os=pc; appver=3.1.28.205001; channel=netease; WEVNSM=1.0.0",
        );
        let response = client
            .eapi_request(LoginRefreshRequest)
            .await
            .expect("eapi login_refresh request failed");

        assert!(matches!(response.code, 200 | 301));
    }
}
