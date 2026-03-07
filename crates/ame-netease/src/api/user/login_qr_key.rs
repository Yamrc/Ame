use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct LoginQrKeyRequest;

impl ApiRequest for LoginQrKeyRequest {
    type Response = Value;

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
        let response: serde_json::Value = client
            .eapi_request(LoginQrKeyRequest)
            .await
            .expect("eapi login_qr_key request failed");

        assert_eq!(response["code"].as_i64(), Some(200));
        assert!(response["unikey"].as_str().is_some());
    }
}
