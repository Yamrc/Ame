use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::common::models::{CreatorDto, TrackDto};
use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PlaylistTrackIdDto {
    pub id: i64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PlaylistDetailDto {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub creator: CreatorDto,
    #[serde(default, rename = "trackCount")]
    pub track_count: u64,
    #[serde(default)]
    pub tracks: Vec<TrackDto>,
    #[serde(default, rename = "trackIds")]
    pub track_ids: Vec<PlaylistTrackIdDto>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PlaylistDetailResponse {
    pub code: i64,
    #[serde(default)]
    pub playlist: PlaylistDetailDto,
}

pub struct PlaylistDetailRequest {
    pub id: i64,
    pub s: u8,
}

impl PlaylistDetailRequest {
    pub fn new(id: i64) -> Self {
        Self { id, s: 0 }
    }
}

impl ApiRequest for PlaylistDetailRequest {
    type Response = PlaylistDetailResponse;

    fn endpoint(&self) -> &'static str {
        "/v6/playlist/detail"
    }

    fn payload(&self) -> Value {
        json!({ "id": self.id, "n": 1000, "s": self.s })
    }
}
