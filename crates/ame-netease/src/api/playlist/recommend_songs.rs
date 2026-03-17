use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::common::models::TrackDto;
use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RecommendSongsData {
    #[serde(default, rename = "dailySongs")]
    pub daily_songs: Vec<TrackDto>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RecommendSongsResponse {
    pub code: i64,
    #[serde(default)]
    pub data: RecommendSongsData,
    #[serde(default, rename = "dailySongs")]
    pub daily_songs: Vec<TrackDto>,
}

pub struct RecommendSongsRequest;

impl RecommendSongsRequest {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RecommendSongsRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequest for RecommendSongsRequest {
    type Response = RecommendSongsResponse;

    fn endpoint(&self) -> &'static str {
        "/api/v3/discovery/recommend/songs"
    }

    fn payload(&self) -> Value {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::RecommendSongsRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn recommend_songs_payload_defaults() {
        let req = RecommendSongsRequest::new();
        assert_eq!(req.endpoint(), "/api/v3/discovery/recommend/songs");
        assert_eq!(req.payload(), json!({}));
    }
}
