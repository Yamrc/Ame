use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoginQrKeyResponse {
    pub code: i64,
    #[serde(default)]
    pub unikey: Option<String>,
}

pub struct LoginQrKeyRequest;

impl ApiRequest for LoginQrKeyRequest {
    type Response = LoginQrKeyResponse;

    fn endpoint(&self) -> &'static str {
        "/api/login/qrcode/unikey"
    }

    fn payload(&self) -> Value {
        json!({
            "type": 3
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::LoginQrKeyRequest;

    #[test]
    fn payload_matches_api_enhanced_shape() {
        let req = LoginQrKeyRequest;
        assert_eq!(req.endpoint(), "/api/login/qrcode/unikey");
        assert_eq!(req.payload()["type"].as_i64(), Some(3));
    }

    #[tokio::test]
    async fn live_login_qr_key_request() {
        let client = crate::NeteaseClient::new();
        let response = client
            .eapi_request(LoginQrKeyRequest)
            .await
            .expect("eapi login_qr_key request failed");

        assert_eq!(response.code, 200);
        assert!(response.unikey.is_some());
    }
}
