use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct PersonalizedPlaylistRequest {
    limit: u32,
}

impl PersonalizedPlaylistRequest {
    pub fn new(limit: u32) -> Self {
        Self { limit }
    }
}

impl ApiRequest for PersonalizedPlaylistRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/api/personalized/playlist"
    }

    fn payload(&self) -> Value {
        json!({
            "limit": self.limit,
            "total": true,
            "n": 1000
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::PersonalizedPlaylistRequest;

    #[test]
    fn personalized_payload_defaults() {
        let req = PersonalizedPlaylistRequest::new(30);
        assert_eq!(req.endpoint(), "/api/personalized/playlist");
        let payload = req.payload();
        assert_eq!(payload["limit"].as_u64(), Some(30));
        assert_eq!(payload["total"].as_bool(), Some(true));
        assert_eq!(payload["n"].as_u64(), Some(1000));
    }
}
