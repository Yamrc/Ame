use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::api::common::models::{UserAccountDto, UserProfileDto};
use crate::api::request::ApiRequest;

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoginStatusData {
    #[serde(default)]
    pub account: Option<UserAccountDto>,
    #[serde(default)]
    pub profile: Option<UserProfileDto>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LoginStatusResponse {
    pub code: i64,
    #[serde(default)]
    pub data: LoginStatusData,
    #[serde(default)]
    pub account: Option<UserAccountDto>,
    #[serde(default)]
    pub profile: Option<UserProfileDto>,
}

impl LoginStatusResponse {
    pub fn account(&self) -> Option<&UserAccountDto> {
        self.data.account.as_ref().or(self.account.as_ref())
    }

    pub fn profile(&self) -> Option<&UserProfileDto> {
        self.data.profile.as_ref().or(self.profile.as_ref())
    }

    pub fn user_id(&self) -> Option<i64> {
        self.account()
            .and_then(|account| account.id)
            .or_else(|| self.profile().and_then(|profile| profile.user_id))
    }
}

pub struct LoginStatusRequest;

impl ApiRequest for LoginStatusRequest {
    type Response = LoginStatusResponse;

    fn endpoint(&self) -> &'static str {
        "/api/w/nuser/account/get"
    }

    fn payload(&self) -> Value {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::LoginStatusRequest;

    #[test]
    fn payload_matches_api_enhanced_status() {
        let req = LoginStatusRequest;
        assert_eq!(req.endpoint(), "/api/w/nuser/account/get");
        assert_eq!(req.payload(), serde_json::json!({}));
    }

    #[tokio::test]
    async fn live_login_status_request() {
        let client = crate::NeteaseClient::with_cookie(
            "os=pc; appver=3.1.28.205001; channel=netease; WEVNSM=1.0.0",
        );
        let response = client
            .weapi_request(LoginStatusRequest)
            .await
            .expect("weapi login_status request failed");

        assert_eq!(response.code, 200);
        assert!(response.account().is_none());
        assert!(response.profile().is_none());
    }
}
