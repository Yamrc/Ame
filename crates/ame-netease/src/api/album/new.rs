use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct AlbumNewRequest {
    limit: u32,
    offset: u32,
    area: String,
}

impl AlbumNewRequest {
    pub fn new(limit: u32, offset: u32, area: impl Into<String>) -> Self {
        Self {
            limit,
            offset,
            area: area.into(),
        }
    }
}

impl ApiRequest for AlbumNewRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/api/album/new"
    }

    fn payload(&self) -> Value {
        json!({
            "limit": self.limit,
            "offset": self.offset,
            "total": true,
            "area": self.area,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::AlbumNewRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn album_new_payload_defaults() {
        let req = AlbumNewRequest::new(30, 0, "ALL");
        assert_eq!(req.endpoint(), "/api/album/new");
        assert_eq!(
            req.payload(),
            json!({
                "limit": 30,
                "offset": 0,
                "total": true,
                "area": "ALL",
            })
        );
    }
}
