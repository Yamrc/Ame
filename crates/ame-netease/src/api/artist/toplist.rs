use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct ToplistArtistRequest {
    artist_type: u32,
    limit: u32,
    offset: u32,
}

impl ToplistArtistRequest {
    pub fn new(artist_type: u32, limit: u32, offset: u32) -> Self {
        Self {
            artist_type,
            limit,
            offset,
        }
    }
}

impl ApiRequest for ToplistArtistRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/api/toplist/artist"
    }

    fn payload(&self) -> Value {
        json!({
            "type": self.artist_type,
            "limit": self.limit,
            "offset": self.offset,
            "total": true,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ToplistArtistRequest;
    use crate::api::request::ApiRequest;

    #[test]
    fn toplist_artist_payload_defaults() {
        let req = ToplistArtistRequest::new(1, 100, 0);
        assert_eq!(req.endpoint(), "/api/toplist/artist");
        assert_eq!(
            req.payload(),
            json!({
                "type": 1,
                "limit": 100,
                "offset": 0,
                "total": true,
            })
        );
    }
}
