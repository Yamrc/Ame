use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct LoginRefreshRequest;

impl ApiRequest for LoginRefreshRequest {
    type Response = Value;

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
}
