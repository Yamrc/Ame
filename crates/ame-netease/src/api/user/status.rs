use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct LoginStatusRequest;

impl ApiRequest for LoginStatusRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/api/w/nuser/account/get"
    }

    fn payload(&self) -> Value {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::LoginStatusRequest;

    #[test]
    fn payload_matches_api_enhanced_status() {
        let req = LoginStatusRequest;
        assert_eq!(req.endpoint(), "/api/w/nuser/account/get");
        assert_eq!(req.payload(), serde_json::json!({}));
    }
}
