use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::common::models::TrackDto;
use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TrackDetailResponse {
    pub code: i64,
    #[serde(default)]
    pub songs: Vec<TrackDto>,
}

pub struct TrackDetailRequest {
    pub ids: Vec<i64>,
}

impl TrackDetailRequest {
    pub fn new(ids: Vec<i64>) -> Self {
        Self { ids }
    }
}

impl ApiRequest for TrackDetailRequest {
    type Response = TrackDetailResponse;

    fn endpoint(&self) -> &'static str {
        "/api/v3/song/detail"
    }

    fn payload(&self) -> Value {
        let c = format!(
            "[{}]",
            self.ids
                .iter()
                .map(|id| format!("{{\"id\":{id}}}"))
                .collect::<Vec<String>>()
                .join(",")
        );
        json!({
            "c": c
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TrackDetailRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn track_detail_payload_uses_c_string() {
        let req = TrackDetailRequest::new(vec![409926, 1384286544]);
        let payload = req.payload();
        assert_eq!(req.endpoint(), "/api/v3/song/detail");
        assert_eq!(
            payload["c"].as_str(),
            Some("[{\"id\":409926},{\"id\":1384286544}]")
        );
    }

    #[tokio::test]
    async fn live_eapi_song_detail_v3_request() {
        let client = crate::NeteaseClient::new();
        let request = TrackDetailRequest::new(vec![409926, 1384286544]);
        let response = client
            .eapi_request(request)
            .await
            .expect("eapi song_detail_v3 request failed");

        assert_eq!(response.code, 200);
        assert!(!response.songs.is_empty());
    }
}
