use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};

use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ToplistArtistSummaryDto {
    #[serde(default, rename = "coverUrl")]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub position: Option<i64>,
    #[serde(default, rename = "upateFrequency")]
    pub upate_frequency: Option<String>,
    #[serde(default, rename = "updateFrequency")]
    pub update_frequency: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ToplistEntryDto {
    pub id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "trackCount")]
    pub track_count: Option<u64>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "ToplistType")]
    pub toplist_type: Option<String>,
    #[serde(default, rename = "coverImgUrl")]
    pub cover_img_url: Option<String>,
    #[serde(default, rename = "updateFrequency")]
    pub update_frequency: Option<String>,
    #[serde(default)]
    pub subscribed: Option<bool>,
    #[serde(default, rename = "specialType")]
    pub special_type: Option<i64>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, rename = "userId")]
    pub user_id: Option<i64>,
    #[serde(default, rename = "playCount")]
    pub play_count: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ToplistResponse {
    pub code: i64,
    pub artist_toplist: Option<ToplistArtistSummaryDto>,
    pub list: Vec<ToplistEntryDto>,
}

impl<'de> Deserialize<'de> for ToplistResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let code = value
            .get("code")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let artist_toplist = value
            .get("artistToplist")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(serde::de::Error::custom)?;
        let list = value
            .get("list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom)?;

        Ok(Self {
            code,
            artist_toplist,
            list,
        })
    }
}

pub struct ToplistRequest;

impl ToplistRequest {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ToplistRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequest for ToplistRequest {
    type Response = ToplistResponse;

    fn endpoint(&self) -> &'static str {
        "/api/toplist"
    }

    fn payload(&self) -> Value {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ToplistRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn toplist_payload_defaults() {
        let req = ToplistRequest::new();
        assert_eq!(req.endpoint(), "/api/toplist");
        assert_eq!(req.payload(), json!({}));
    }

    #[tokio::test]
    async fn live_toplist_request() {
        let client = crate::NeteaseClient::new();
        let response = client
            .eapi_request(ToplistRequest::new())
            .await
            .expect("toplist request failed");

        assert_eq!(response.code, 200);
        assert!(!response.list.is_empty());
    }
}
