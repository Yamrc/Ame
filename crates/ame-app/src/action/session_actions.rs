use ame_core::secure::CredentialStore;
use ame_netease::api::user::status::LoginStatusRequest;
use ame_netease::{ClientError, NeteaseClient};
use anyhow::{Result, anyhow};

pub fn save_cookie(store: &CredentialStore, cookie: &str) -> Result<()> {
    store.save_cookie(cookie).map_err(|e| anyhow!(e.to_string()))
}

pub fn load_cookie(store: &CredentialStore) -> Result<Option<String>> {
    store.load_cookie().map_err(|e| anyhow!(e.to_string()))
}

pub async fn verify_login(cookie: &str) -> Result<bool> {
    let client = NeteaseClient::with_cookie(cookie);
    let resp: Result<serde_json::Value, ClientError> = client
        .weapi_request(LoginStatusRequest::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as i64),
        ))
        .await;
    match resp {
        Ok(value) => Ok(value["data"]["account"]["id"].as_i64().is_some()),
        Err(_) => Ok(false),
    }
}
