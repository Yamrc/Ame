use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct UserPlaylistRequest {
    pub uid: i64,
    pub offset: u32,
    pub limit: u32,
}

impl UserPlaylistRequest {
    pub fn new(uid: i64) -> Self {
        Self {
            uid,
            offset: 0,
            limit: 200,
        }
    }
}

impl ApiRequest for UserPlaylistRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/user/playlist"
    }

    fn payload(&self) -> Value {
        json!({
            "uid": self.uid,
            "offset": self.offset,
            "limit": self.limit,
            "includeVideo": true
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::UserPlaylistRequest;

    #[test]
    fn payload_contains_uid_and_pagination() {
        let req = UserPlaylistRequest::new(123);
        let payload = req.payload();
        assert_eq!(req.endpoint(), "/user/playlist");
        assert_eq!(payload["uid"].as_i64(), Some(123));
        assert_eq!(payload["offset"].as_u64(), Some(0));
        assert_eq!(payload["limit"].as_u64(), Some(200));
        assert_eq!(payload["includeVideo"].as_bool(), Some(true));
    }
}
