use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct PlaylistListRequest {
    cat: String,
    order: String,
    limit: u32,
    offset: u32,
}

impl PlaylistListRequest {
    pub fn new(limit: u32, offset: u32) -> Self {
        Self {
            cat: "全部".to_string(),
            order: "hot".to_string(),
            limit,
            offset,
        }
    }

    pub fn with_cat(mut self, cat: impl Into<String>) -> Self {
        self.cat = cat.into();
        self
    }

    pub fn with_order(mut self, order: impl Into<String>) -> Self {
        self.order = order.into();
        self
    }
}

impl ApiRequest for PlaylistListRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/playlist/list"
    }

    fn payload(&self) -> Value {
        json!({
            "cat": self.cat,
            "order": self.order,
            "limit": self.limit,
            "offset": self.offset,
            "total": true
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::PlaylistListRequest;

    #[test]
    fn playlist_list_payload_defaults() {
        let req = PlaylistListRequest::new(30, 0);
        assert_eq!(req.endpoint(), "/playlist/list");
        let payload = req.payload();
        assert_eq!(payload["cat"].as_str(), Some("全部"));
        assert_eq!(payload["order"].as_str(), Some("hot"));
        assert_eq!(payload["limit"].as_u64(), Some(30));
        assert_eq!(payload["offset"].as_u64(), Some(0));
    }
}
