use ame_core::credential::AuthBundle;
use ame_netease::NeteaseClient;
use ame_netease::api::user::login_qr_check::LoginQrCheckRequest;
use ame_netease::api::user::login_qr_key::LoginQrKeyRequest;
use ame_netease::api::user::login_refresh::LoginRefreshRequest;
use ame_netease::api::user::register_anonymous::RegisterAnonymousRequest;
use ame_netease::api::user::status::LoginStatusRequest;
use anyhow::{Context as _, Result};
use serde_json::Value;
use std::future::Future;

const MUSIC_U: &str = "MUSIC_U";
const MUSIC_A: &str = "MUSIC_A";
const CSRF: &str = "__csrf";
const MUSIC_R_T: &str = "MUSIC_R_T";

#[derive(Debug, Clone)]
pub struct ResponseWithCookies {
    pub body: Value,
    pub set_cookie: Vec<String>,
}

fn block_on<F, T, E>(future: F) -> Result<T>
where
    F: Future<Output = std::result::Result<T, E>>,
    E: std::error::Error + Send + Sync + 'static,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build temporary tokio runtime")?;
    Ok(runtime.block_on(future)?)
}

pub fn build_cookie_header(bundle: &AuthBundle) -> Option<String> {
    let mut pairs = Vec::new();

    if let Some(music_u) = bundle.music_u.as_ref().filter(|it| !it.trim().is_empty()) {
        pairs.push(format!("{MUSIC_U}={music_u}"));
    }
    if let Some(music_a) = bundle.music_a.as_ref().filter(|it| !it.trim().is_empty()) {
        pairs.push(format!("{MUSIC_A}={music_a}"));
    }
    if let Some(csrf) = bundle.csrf.as_ref().filter(|it| !it.trim().is_empty()) {
        pairs.push(format!("{CSRF}={csrf}"));
    }
    if let Some(music_r_t) = bundle.music_r_t.as_ref().filter(|it| !it.trim().is_empty()) {
        pairs.push(format!("{MUSIC_R_T}={music_r_t}"));
    }

    if pairs.is_empty() {
        return None;
    }
    Some(pairs.join("; "))
}

pub fn merge_bundle_from_set_cookie(bundle: &mut AuthBundle, set_cookie: &[String]) -> bool {
    let mut changed = false;

    for raw in set_cookie {
        if let Some((key, value)) = parse_cookie_fragment(raw) {
            match key.as_str() {
                MUSIC_U => changed |= replace_if_changed(&mut bundle.music_u, value),
                MUSIC_A => changed |= replace_if_changed(&mut bundle.music_a, value),
                CSRF => changed |= replace_if_changed(&mut bundle.csrf, value),
                MUSIC_R_T => changed |= replace_if_changed(&mut bundle.music_r_t, value),
                _ => {}
            }
        }
    }

    changed
}

pub fn fetch_login_qr_key_blocking(cookie: Option<&str>) -> Result<ResponseWithCookies> {
    let client = cookie
        .filter(|it| !it.trim().is_empty())
        .map_or_else(NeteaseClient::new, NeteaseClient::with_cookie);

    let body = block_on(client.eapi_request(LoginQrKeyRequest))?;
    let mut set_cookie = client.take_last_set_cookie();
    set_cookie.extend(extract_cookie_from_body(&body));
    Ok(ResponseWithCookies { body, set_cookie })
}

pub fn check_login_qr_blocking(key: &str, cookie: Option<&str>) -> Result<ResponseWithCookies> {
    let client = cookie
        .filter(|it| !it.trim().is_empty())
        .map_or_else(NeteaseClient::new, NeteaseClient::with_cookie);

    let body = block_on(client.eapi_request(LoginQrCheckRequest::new(key)))?;
    let mut set_cookie = client.take_last_set_cookie();
    set_cookie.extend(extract_cookie_from_body(&body));
    Ok(ResponseWithCookies { body, set_cookie })
}

pub fn register_anonymous_blocking(cookie: Option<&str>) -> Result<ResponseWithCookies> {
    let cookie = cookie
        .filter(|it| !it.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            "os=pc; appver=3.1.28.205001; channel=netease; WEVNSM=1.0.0".to_string()
        });
    let client = NeteaseClient::with_cookie(cookie);

    let body = block_on(client.weapi_request(RegisterAnonymousRequest::new()))?;
    let mut set_cookie = client.take_last_set_cookie();
    set_cookie.extend(extract_cookie_from_body(&body));
    Ok(ResponseWithCookies { body, set_cookie })
}

pub fn fetch_login_status_blocking(cookie: Option<&str>) -> Result<Value> {
    let client = cookie
        .filter(|it| !it.trim().is_empty())
        .map_or_else(NeteaseClient::new, NeteaseClient::with_cookie);
    block_on(client.weapi_request(LoginStatusRequest))
}

pub fn refresh_login_token_blocking(cookie: Option<&str>) -> Result<ResponseWithCookies> {
    let client = cookie
        .filter(|it| !it.trim().is_empty())
        .map_or_else(NeteaseClient::new, NeteaseClient::with_cookie);
    let body = block_on(client.eapi_request(LoginRefreshRequest))?;
    let mut set_cookie = client.take_last_set_cookie();
    set_cookie.extend(extract_cookie_from_body(&body));
    Ok(ResponseWithCookies { body, set_cookie })
}

pub fn login_summary_text(value: &Value) -> Option<String> {
    let profile = &value["data"]["profile"];
    let nickname = profile["nickname"].as_str().unwrap_or_default();
    let user_id = profile["userId"].as_i64().unwrap_or_default();
    if !nickname.is_empty() && user_id > 0 {
        return Some(format!("{nickname} (#{user_id})"));
    }
    None
}

fn replace_if_changed(slot: &mut Option<String>, value: String) -> bool {
    if slot.as_ref() == Some(&value) {
        return false;
    }
    *slot = Some(value);
    true
}

fn parse_cookie_fragment(raw: &str) -> Option<(String, String)> {
    let first = raw.split(';').next()?.trim();
    let (key, value) = first.split_once('=')?;
    if key.trim().is_empty() || value.trim().is_empty() {
        return None;
    }
    Some((key.trim().to_string(), value.trim().to_string()))
}

fn extract_cookie_from_body(body: &Value) -> Vec<String> {
    if let Some(cookie) = body.get("cookie").and_then(Value::as_str) {
        return cookie
            .split(";;")
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use ame_core::credential::AuthBundle;

    use super::{build_cookie_header, merge_bundle_from_set_cookie};

    #[test]
    fn cookie_header_prioritizes_music_u_and_keeps_music_a() {
        let bundle = AuthBundle {
            music_u: Some("u".to_string()),
            music_a: Some("a".to_string()),
            csrf: Some("c".to_string()),
            music_r_t: None,
        };

        assert_eq!(
            build_cookie_header(&bundle).as_deref(),
            Some("MUSIC_U=u; MUSIC_A=a; __csrf=c")
        );
    }

    #[test]
    fn merge_only_updates_whitelisted_keys() {
        let mut bundle = AuthBundle::default();
        let changed = merge_bundle_from_set_cookie(
            &mut bundle,
            &[
                "MUSIC_A=guest; Path=/; HttpOnly".to_string(),
                "SID=ignored; Path=/".to_string(),
                "__csrf=token; Path=/".to_string(),
            ],
        );

        assert!(changed);
        assert_eq!(bundle.music_a.as_deref(), Some("guest"));
        assert_eq!(bundle.csrf.as_deref(), Some("token"));
        assert_eq!(bundle.music_u, None);
    }
}
