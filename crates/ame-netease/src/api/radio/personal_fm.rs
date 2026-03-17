use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::common::models::TrackDto;
use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PersonalFmResponse {
    pub code: i64,
    #[serde(default)]
    pub data: Vec<TrackDto>,
}

pub struct PersonalFmRequest;

impl PersonalFmRequest {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PersonalFmRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequest for PersonalFmRequest {
    type Response = PersonalFmResponse;

    fn endpoint(&self) -> &'static str {
        "/api/v1/radio/get"
    }

    fn payload(&self) -> Value {
        Value::Object(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::PersonalFmRequest;

    #[test]
    fn personal_fm_payload_defaults() {
        let req = PersonalFmRequest::new();
        assert_eq!(req.endpoint(), "/api/v1/radio/get");
        let payload = req.payload();
        assert!(payload.as_object().is_some());
    }
}
