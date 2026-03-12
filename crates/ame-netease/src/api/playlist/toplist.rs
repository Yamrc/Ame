use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct ToplistRequest;

impl ToplistRequest {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ToplistRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequest for ToplistRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/api/toplist"
    }

    fn payload(&self) -> Value {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ToplistRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn toplist_payload_defaults() {
        let req = ToplistRequest::new();
        assert_eq!(req.endpoint(), "/api/toplist");
        assert_eq!(req.payload(), json!({}));
    }
}
