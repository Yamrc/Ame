use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct LoginStatusRequest {
    pub timestamp: i64,
}

impl LoginStatusRequest {
    pub fn new(timestamp: i64) -> Self {
        Self { timestamp }
    }
}

impl ApiRequest for LoginStatusRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/login/status"
    }

    fn payload(&self) -> Value {
        json!({ "timestamp": self.timestamp })
    }
}
