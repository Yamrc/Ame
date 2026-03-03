use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct TrackDetailRequest {
    pub ids: Vec<i64>,
}

impl TrackDetailRequest {
    pub fn new(ids: Vec<i64>) -> Self {
        Self { ids }
    }
}

impl ApiRequest for TrackDetailRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/song/detail"
    }

    fn payload(&self) -> Value {
        json!({
            "c": self
                .ids
                .iter()
                .map(|id| json!({ "id": id }))
                .collect::<Vec<Value>>()
        })
    }
}
