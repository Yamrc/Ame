use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::common::models::PlaylistDto;
use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RecommendResourceResponse {
    pub code: i64,
    #[serde(default, rename = "recommend")]
    pub playlists: Vec<PlaylistDto>,
}

pub struct RecommendResourceRequest;

impl RecommendResourceRequest {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RecommendResourceRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequest for RecommendResourceRequest {
    type Response = RecommendResourceResponse;

    fn endpoint(&self) -> &'static str {
        "/api/v1/discovery/recommend/resource"
    }

    fn payload(&self) -> Value {
        Value::Object(Default::default())
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::RecommendResourceRequest;

    #[test]
    fn recommend_resource_payload_defaults() {
        let req = RecommendResourceRequest::new();
        assert_eq!(req.endpoint(), "/api/v1/discovery/recommend/resource");
        let payload = req.payload();
        assert!(payload.as_object().is_some());
    }
}
