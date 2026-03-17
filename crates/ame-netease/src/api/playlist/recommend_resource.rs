use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::api::common::models::PlaylistDto;
use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Serialize)]
pub struct RecommendResourceResponse {
    pub code: i64,
    pub playlists: Vec<PlaylistDto>,
}

impl<'de> Deserialize<'de> for RecommendResourceResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let code = value
            .get("code")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let playlists = value
            .get("recommend")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom)?;

        Ok(Self { code, playlists })
    }
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

    use super::{RecommendResourceRequest, RecommendResourceResponse};

    #[test]
    fn recommend_resource_payload_defaults() {
        let req = RecommendResourceRequest::new();
        assert_eq!(req.endpoint(), "/api/v1/discovery/recommend/resource");
        let payload = req.payload();
        assert!(payload.as_object().is_some());
    }

    #[test]
    fn recommend_resource_allows_duplicate_creator_keys() {
        let raw = r#"{
            "code":200,
            "recommend":[
                {
                    "id":1,
                    "name":"test",
                    "trackCount":3,
                    "creator":{"nickname":"first","userId":1},
                    "creator":{"nickname":"second","userId":2},
                    "coverImgUrl":"http://example.com/a.jpg"
                }
            ]
        }"#;
        let response: RecommendResourceResponse =
            serde_json::from_str(raw).expect("response should parse with duplicate keys");
        assert_eq!(response.code, 200);
        assert_eq!(response.playlists.len(), 1);
        assert_eq!(response.playlists[0].id, 1);
        assert_eq!(
            response.playlists[0]
                .creator
                .as_ref()
                .and_then(|creator| creator.user_id),
            Some(2)
        );
    }
}
