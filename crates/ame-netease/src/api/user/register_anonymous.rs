use base64::{Engine as _, engine::general_purpose::STANDARD};
use md5::{Digest, Md5};
use rand::RngExt;
use serde_json::{Value, json};

use crate::api::request::ApiRequest;

const ID_XOR_KEY_1: &[u8] = b"3go8&$8*3*3h0k(2)2";

pub struct RegisterAnonymousRequest {
    pub username: String,
}

impl RegisterAnonymousRequest {
    pub fn new() -> Self {
        let device_id = generate_device_id();
        let encoded = cloudmusic_dll_encode_id(&device_id);
        let username = STANDARD.encode(format!("{device_id} {encoded}").as_bytes());
        Self { username }
    }
}

impl Default for RegisterAnonymousRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequest for RegisterAnonymousRequest {
    type Response = Value;

    fn endpoint(&self) -> &'static str {
        "/api/register/anonimous"
    }

    fn payload(&self) -> Value {
        json!({
            "username": self.username
        })
    }
}

fn generate_device_id() -> String {
    const HEX: &[u8] = b"0123456789ABCDEF";
    let mut bytes = [0_u8; 52];
    rand::rng().fill(&mut bytes);
    bytes
        .into_iter()
        .map(|value| HEX[(value % 16) as usize] as char)
        .collect()
}

fn cloudmusic_dll_encode_id(raw: &str) -> String {
    let xored: Vec<u8> = raw
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(idx, byte)| byte ^ ID_XOR_KEY_1[idx % ID_XOR_KEY_1.len()])
        .collect();
    let mut md5 = Md5::new();
    md5.update(&xored);
    STANDARD.encode(md5.finalize())
}

#[cfg(test)]
mod tests {
    use crate::api::request::ApiRequest;

    use super::RegisterAnonymousRequest;

    #[test]
    fn payload_contains_generated_username() {
        let req = RegisterAnonymousRequest::new();
        assert_eq!(req.endpoint(), "/api/register/anonimous");
        assert!(
            req.payload()["username"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
    }

    #[tokio::test]
    async fn live_register_anonymous_request() {
        let client = crate::NeteaseClient::with_cookie(
            "os=pc; appver=3.1.28.205001; channel=netease; WEVNSM=1.0.0",
        );
        let response: serde_json::Value = client
            .weapi_request(RegisterAnonymousRequest::new())
            .await
            .expect("weapi register_anonymous request failed");
        let set_cookie = client.take_last_set_cookie();
        let has_music_a = set_cookie.iter().any(|value| value.starts_with("MUSIC_A="));
        let has_nmtid = set_cookie.iter().any(|value| value.starts_with("NMTID="));
        let status_code = response["code"].as_i64();

        assert!(
            status_code == Some(200) || status_code == Some(400) || has_music_a || has_nmtid,
            "unexpected register anonymous response: {response}, set-cookie: {set_cookie:?}"
        );
    }
}
