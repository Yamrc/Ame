use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct LoginQrCheckRequest {
    pub key: String,
}

impl LoginQrCheckRequest {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

impl ApiRequest for LoginQrCheckRequest {
    type Response = Value;

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
}
