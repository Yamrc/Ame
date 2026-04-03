use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LikeTrackResponse {
    pub code: i64,
}

pub struct LikeTrackRequest {
    pub track_id: i64,
    pub like: bool,
    pub csrf_token: String,
}

impl LikeTrackRequest {
    pub fn new(track_id: i64, like: bool, csrf_token: impl Into<String>) -> Self {
        Self {
            track_id,
            like,
            csrf_token: csrf_token.into(),
        }
    }
}

impl ApiRequest for LikeTrackRequest {
    type Response = LikeTrackResponse;

    fn endpoint(&self) -> &'static str {
        "/api/radio/like"
    }

    fn payload(&self) -> Value {
        json!({
            "alg": "itembased",
            "trackId": self.track_id,
            "like": self.like,
            "time": "3",
            "csrf_token": self.csrf_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::LikeTrackRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn like_track_payload_matches_api_enhanced() {
        let req = LikeTrackRequest::new(123, true, "csrf");
        let payload = req.payload();
        assert_eq!(req.endpoint(), "/api/radio/like");
        assert_eq!(payload["alg"].as_str(), Some("itembased"));
        assert_eq!(payload["trackId"].as_i64(), Some(123));
        assert_eq!(payload["like"].as_bool(), Some(true));
        assert_eq!(payload["time"].as_str(), Some("3"));
        assert_eq!(payload["csrf_token"].as_str(), Some("csrf"));
    }
}
