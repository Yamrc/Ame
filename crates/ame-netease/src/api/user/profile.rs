use serde_json::{Value, json};

use crate::api::request::ApiRequest;

pub struct UserAccountRequest;

impl ApiRequest for UserAccountRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/user/account"
    }

    fn payload(&self) -> Value {
        json!({})
    }
}
