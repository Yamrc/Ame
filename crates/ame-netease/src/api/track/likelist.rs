use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LikedTrackListData {
    #[serde(default)]
    pub ids: Vec<i64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LikedTrackListResponse {
    pub code: i64,
    #[serde(default)]
    pub ids: Vec<i64>,
    #[serde(default)]
    pub data: Option<LikedTrackListData>,
}

impl LikedTrackListResponse {
    pub fn ids(&self) -> &[i64] {
        self.data
            .as_ref()
            .filter(|data| !data.ids.is_empty())
            .map(|data| data.ids.as_slice())
            .unwrap_or(self.ids.as_slice())
    }
}

pub struct LikedTrackListRequest {
    pub uid: i64,
}

impl LikedTrackListRequest {
    pub fn new(uid: i64) -> Self {
        Self { uid }
    }
}

impl ApiRequest for LikedTrackListRequest {
    type Response = LikedTrackListResponse;

    fn endpoint(&self) -> &'static str {
        "/api/song/like/get"
    }

    fn payload(&self) -> Value {
        json!({
            "uid": self.uid,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{LikedTrackListRequest, LikedTrackListResponse};
    use crate::api::request::ApiRequest;

    #[test]
    fn liked_track_list_payload_matches_api_enhanced() {
        let req = LikedTrackListRequest::new(42);
        assert_eq!(req.endpoint(), "/api/song/like/get");
        assert_eq!(req.payload(), json!({ "uid": 42 }));
    }

    #[test]
    fn liked_track_list_prefers_nested_ids_when_present() {
        let response: LikedTrackListResponse = serde_json::from_value(json!({
            "code": 200,
            "ids": [1, 2],
            "data": {
                "ids": [3, 4]
            }
        }))
        .expect("response should deserialize");

        assert_eq!(response.ids(), &[3, 4]);
    }
}
