use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LyricDto {
    #[serde(default)]
    pub lyric: Option<String>,
    #[serde(default)]
    pub version: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TrackLyricResponse {
    pub code: i64,
    #[serde(default)]
    pub lrc: LyricDto,
    #[serde(default)]
    pub tlyric: LyricDto,
    #[serde(default)]
    pub klyric: LyricDto,
    #[serde(default)]
    pub qfy: Option<bool>,
    #[serde(default)]
    pub sfy: Option<bool>,
    #[serde(default)]
    pub sgc: Option<bool>,
}

impl TrackLyricResponse {
    pub fn main_lyric(&self) -> Option<&str> {
        self.lrc
            .lyric
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }
}

pub struct TrackLyricRequest {
    pub id: i64,
}

impl TrackLyricRequest {
    pub fn new(id: i64) -> Self {
        Self { id }
    }
}

impl ApiRequest for TrackLyricRequest {
    type Response = TrackLyricResponse;

    fn endpoint(&self) -> &'static str {
        "/song/lyric"
    }

    fn payload(&self) -> Value {
        json!({ "id": self.id, "lv": -1, "kv": -1, "tv": -1 })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::TrackLyricRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn lyric_payload_defaults() {
        let req = TrackLyricRequest::new(409926);
        assert_eq!(req.endpoint(), "/song/lyric");
        assert_eq!(
            req.payload(),
            json!({ "id": 409926, "lv": -1, "kv": -1, "tv": -1 })
        );
    }

    #[tokio::test]
    async fn live_track_lyric_request() {
        let client = crate::NeteaseClient::new();
        let response = client
            .weapi_request(TrackLyricRequest::new(409926))
            .await
            .expect("weapi track_lyric request failed");

        assert_eq!(response.code, 200);
        assert!(response.main_lyric().is_some());
    }
}
