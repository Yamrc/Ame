use serde_json::{Value, json};

use crate::api::request::ApiRequest;

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
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/v6/playlist/detail"
    }

    fn payload(&self) -> Value {
        json!({ "id": self.id, "n": 1000, "s": self.s })
    }
}
