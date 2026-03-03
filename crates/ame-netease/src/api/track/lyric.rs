use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct TrackLyricRequest {
    pub id: i64,
}

impl TrackLyricRequest {
    pub fn new(id: i64) -> Self {
        Self { id }
    }
}

impl ApiRequest for TrackLyricRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/song/lyric"
    }

    fn payload(&self) -> Value {
        json!({ "id": self.id, "lv": -1, "kv": -1, "tv": -1 })
    }
}
