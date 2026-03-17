use serde_json::{Value, json};

use super::status::LoginStatusResponse;
use crate::api::request::ApiRequest;

pub type UserAccountResponse = LoginStatusResponse;

pub struct UserAccountRequest;

impl ApiRequest for UserAccountRequest {
    type Response = UserAccountResponse;

    fn endpoint(&self) -> &'static str {
        "/api/nuser/account/get"
    }

    fn payload(&self) -> Value {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::UserAccountRequest;

    #[test]
    fn payload_matches_user_account_shape() {
        let req = UserAccountRequest;
        assert_eq!(req.endpoint(), "/api/nuser/account/get");
        assert_eq!(req.payload(), serde_json::json!({}));
    }

    #[tokio::test]
    async fn live_user_account_request() {
        let client = crate::NeteaseClient::with_cookie(
            "os=pc; appver=3.1.28.205001; channel=netease; WEVNSM=1.0.0",
        );
        let response = client
            .weapi_request(UserAccountRequest)
            .await
            .expect("weapi user_account request failed");

        assert_eq!(response.code, 200);
    }
}
