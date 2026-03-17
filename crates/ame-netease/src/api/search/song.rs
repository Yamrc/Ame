use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::common::models::TrackDto;
use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SearchSongResult {
    #[serde(default)]
    pub songs: Vec<TrackDto>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SearchSongResponse {
    pub code: i64,
    #[serde(default)]
    pub result: SearchSongResult,
}

pub struct SearchSongRequest {
    pub keywords: String,
    pub offset: u32,
    pub limit: u32,
}

impl SearchSongRequest {
    pub fn new(keywords: impl Into<String>) -> Self {
        Self {
            keywords: keywords.into(),
            offset: 0,
            limit: 30,
        }
    }
}

impl ApiRequest for SearchSongRequest {
    type Response = SearchSongResponse;

    fn endpoint(&self) -> &'static str {
        "/api/search/get"
    }

    fn payload(&self) -> Value {
        json!({
            "s": self.keywords,
            "type": 1,
            "offset": self.offset,
            "limit": self.limit
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SearchSongRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn search_payload_contains_keyword() {
        let req = SearchSongRequest::new("hello");
        let payload = req.payload();
        assert_eq!(req.endpoint(), "/api/search/get");
        assert_eq!(payload["s"].as_str(), Some("hello"));
        assert_eq!(payload["type"].as_i64(), Some(1));
    }

    #[tokio::test]
    async fn live_weapi_search_request() {
        let client = crate::NeteaseClient::new();
        let request = SearchSongRequest::new("夕日坂");
        let response = client
            .weapi_request(request)
            .await
            .expect("weapi search request failed");

        assert_eq!(response.code, 200);
        assert!(!response.result.songs.is_empty());
    }
}
