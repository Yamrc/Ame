use serde_json::Value;

use crate::api::request::ApiRequest;

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
    type Response = Value;

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
