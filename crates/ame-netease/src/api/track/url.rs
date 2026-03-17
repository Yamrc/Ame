use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TrackFreeTrialPrivilegeDto {
    #[serde(default, rename = "cannotListenReason")]
    pub cannot_listen_reason: Option<i64>,
    #[serde(default, rename = "freeLimitTagType")]
    pub free_limit_tag_type: Option<i64>,
    #[serde(default, rename = "listenType")]
    pub listen_type: Option<i64>,
    #[serde(default, rename = "playReason")]
    pub play_reason: Option<String>,
    #[serde(default, rename = "resConsumable")]
    pub res_consumable: Option<bool>,
    #[serde(default, rename = "userConsumable")]
    pub user_consumable: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TrackFreeTimeTrialPrivilegeDto {
    #[serde(default, rename = "remainTime")]
    pub remain_time: Option<u64>,
    #[serde(default, rename = "resConsumable")]
    pub res_consumable: Option<bool>,
    #[serde(default)]
    pub r#type: Option<i64>,
    #[serde(default, rename = "userConsumable")]
    pub user_consumable: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TrackUrlDto {
    pub id: i64,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub code: Option<i64>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub fee: Option<i64>,
    #[serde(default)]
    pub payed: Option<i64>,
    #[serde(default, rename = "encodeType")]
    pub encode_type: Option<String>,
    #[serde(default)]
    pub br: Option<u64>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub time: Option<u64>,
    #[serde(default, rename = "freeTrialPrivilege")]
    pub free_trial_privilege: Option<TrackFreeTrialPrivilegeDto>,
    #[serde(default, rename = "freeTimeTrialPrivilege")]
    pub free_time_trial_privilege: Option<TrackFreeTimeTrialPrivilegeDto>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TrackUrlResponse {
    pub code: i64,
    #[serde(default)]
    pub data: Vec<TrackUrlDto>,
}

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
        Self { ids, level }
    }
}

impl ApiRequest for TrackUrlRequest {
    type Response = TrackUrlResponse;

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
        let req = TrackUrlRequest::with_level(vec![409926], "sky".to_string());
        let payload = req.payload();
        assert_eq!(payload["immerseType"].as_str(), Some("c51"));
    }

    #[tokio::test]
    async fn live_eapi_song_url_v1_request() {
        let client = crate::NeteaseClient::new();
        let request = TrackUrlRequest::new(vec![409926, 1384286544]);
        let response = client
            .eapi_request(request)
            .await
            .expect("eapi song_url_v1 request failed");

        assert_eq!(response.code, 200);
        assert!(!response.data.is_empty());
        assert!(response.data[0].id > 0);
    }
}
