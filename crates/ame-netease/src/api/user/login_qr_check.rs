use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoginQrCheckResponse {
    pub code: i64,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub msg: Option<String>,
    #[serde(default)]
    pub cookie: Option<String>,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default, rename = "avatarUrl")]
    pub avatar_url: Option<String>,
}

pub struct LoginQrCheckRequest {
    pub key: String,
}

impl LoginQrCheckRequest {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl ApiRequest for LoginQrCheckRequest {
    type Response = LoginQrCheckResponse;

    fn endpoint(&self) -> &'static str {
        "/api/login/qrcode/client/login"
    }

    fn payload(&self) -> Value {
        json!({
            "key": self.key,
            "type": 3
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::LoginQrCheckRequest;

    #[test]
    fn payload_contains_key_and_type() {
        let req = LoginQrCheckRequest::new("abc");
        assert_eq!(req.endpoint(), "/api/login/qrcode/client/login");
        assert_eq!(req.payload()["key"].as_str(), Some("abc"));
        assert_eq!(req.payload()["type"].as_i64(), Some(3));
    }

    #[tokio::test]
    async fn live_login_qr_check_request() {
        let client = crate::NeteaseClient::new();
        let key_response = client
            .eapi_request(crate::api::user::login_qr_key::LoginQrKeyRequest)
            .await
            .expect("eapi login_qr_key request failed");
        let response = client
            .eapi_request(LoginQrCheckRequest::new(
                key_response.unikey.unwrap_or_default(),
            ))
            .await
            .expect("eapi login_qr_check request failed");

        assert!(matches!(response.code, 800..=803));
    }
}
