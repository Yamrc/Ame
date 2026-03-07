use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct TrackUrlRequest {
    pub ids: Vec<i64>,
    pub level: String,
}

impl TrackUrlRequest {
    pub fn new(ids: Vec<i64>) -> Self {
        Self {
            ids,
            level: "exhigh".to_string(),
        }
    }

    pub fn with_level(ids: Vec<i64>, level: String) -> Self {
        Self {
            ids,
            level,
        }
    }
}

impl ApiRequest for TrackUrlRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/api/song/enhance/player/url/v1"
    }

    fn payload(&self) -> Value {
        let mut payload = json!({
            "ids": format!(
                "[{}]",
                self.ids
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            "level": self.level,
            "encodeType": "flac"
        });

        if self.level == "sky" {
            payload["immerseType"] = json!("c51");
        }

        payload
    }
}

#[cfg(test)]
mod tests {
    use super::TrackUrlRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn payload_uses_song_url_v1_shape() {
        let req = TrackUrlRequest::new(vec![409926, 1384286544]);
        let payload = req.payload();
        assert_eq!(req.endpoint(), "/api/song/enhance/player/url/v1");
        assert_eq!(payload["ids"].as_str(), Some("[409926,1384286544]"));
        assert_eq!(payload["level"].as_str(), Some("exhigh"));
        assert_eq!(payload["encodeType"].as_str(), Some("flac"));
    }

    #[test]
    fn sky_level_sets_immerse_type() {
        let req = TrackUrlRequest::with_level(vec![409926], "sky");
        let payload = req.payload();
        assert_eq!(payload["immerseType"].as_str(), Some("c51"));
    }

    #[tokio::test]
    async fn live_eapi_song_url_v1_request() {
        let client = crate::NeteaseClient::new();
        let request = TrackUrlRequest::new(vec![409926, 1384286544]);
        let response: serde_json::Value = client
            .eapi_request(request)
            .await
            .expect("eapi song_url_v1 request failed");

        assert_eq!(response["code"].as_i64(), Some(200));
        assert!(response["data"].is_array());
        assert!(response["data"][0]["id"].as_i64().is_some());

        // println!(
        //     "{:?}, {:?}",
        //     response["data"][0]["url"], response["data"][1]["url"]
        // )
    }
}
