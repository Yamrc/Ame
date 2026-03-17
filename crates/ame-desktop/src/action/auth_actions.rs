use ame_core::credential::AuthBundle;
use ame_netease::NeteaseClient;
use ame_netease::api::common::models::UserProfileDto;
use ame_netease::api::user::login_qr_check::LoginQrCheckRequest;
use ame_netease::api::user::login_qr_check::LoginQrCheckResponse;
use ame_netease::api::user::login_qr_key::LoginQrKeyRequest;
use ame_netease::api::user::login_qr_key::LoginQrKeyResponse;
use ame_netease::api::user::login_refresh::LoginRefreshRequest;
use ame_netease::api::user::login_refresh::LoginRefreshResponse;
use ame_netease::api::user::register_anonymous::RegisterAnonymousRequest;
use ame_netease::api::user::register_anonymous::RegisterAnonymousResponse;
use ame_netease::api::user::status::LoginStatusRequest;
use ame_netease::api::user::status::LoginStatusResponse;
use anyhow::Result;

use crate::action::runtime::{block_on, netease_client};

const MUSIC_U: &str = "MUSIC_U";
const MUSIC_A: &str = "MUSIC_A";
const CSRF: &str = "__csrf";
const MUSIC_R_T: &str = "MUSIC_R_T";

#[derive(Debug, Clone)]
pub struct ResponseWithCookies<T> {
    pub body: T,
    pub set_cookie: Vec<String>,
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

pub fn fetch_login_qr_key_blocking(
    cookie: Option<&str>,
) -> Result<ResponseWithCookies<LoginQrKeyResponse>> {
    let client = netease_client(cookie);
    let body = block_on(client.eapi_request(LoginQrKeyRequest))?;
    Ok(response_with_cookies(&client, body, None))
}

pub fn check_login_qr_blocking(
    key: &str,
    cookie: Option<&str>,
) -> Result<ResponseWithCookies<LoginQrCheckResponse>> {
    let client = netease_client(cookie);
    let body = block_on(client.eapi_request(LoginQrCheckRequest::new(key)))?;
    let cookie = body.cookie.clone();
    Ok(response_with_cookies(&client, body, cookie.as_deref()))
}

pub fn register_anonymous_blocking(
    cookie: Option<&str>,
) -> Result<ResponseWithCookies<RegisterAnonymousResponse>> {
    let cookie = cookie
        .filter(|it| !it.trim().is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            "os=pc; appver=3.1.28.205001; channel=netease; WEVNSM=1.0.0".to_string()
        });
    let client = NeteaseClient::with_cookie(cookie);

    let body = block_on(client.weapi_request(RegisterAnonymousRequest::new()))?;
    let mut set_cookie = client.take_last_set_cookie();
    set_cookie.extend(extract_cookie_from_body(body.cookie.as_deref()));
    Ok(ResponseWithCookies { body, set_cookie })
}

pub fn fetch_login_status_blocking(cookie: Option<&str>) -> Result<LoginStatusResponse> {
    let client = netease_client(cookie);
    block_on(client.weapi_request(LoginStatusRequest))
}

pub fn refresh_login_token_blocking(
    cookie: Option<&str>,
) -> Result<ResponseWithCookies<LoginRefreshResponse>> {
    let client = netease_client(cookie);
    let body = block_on(client.eapi_request(LoginRefreshRequest))?;
    let cookie = body.cookie.clone();
    Ok(response_with_cookies(&client, body, cookie.as_deref()))
}

pub fn login_summary_text(value: &LoginStatusResponse) -> Option<String> {
    let profile = value.profile()?;
    let nickname = profile.nickname.as_deref().unwrap_or_default();
    let user_id = profile.user_id.unwrap_or_default();
    if !nickname.is_empty() && user_id > 0 {
        return Some(format!("{nickname} (#{user_id})"));
    }
    None
}

pub fn login_profile(value: &LoginStatusResponse) -> Option<&UserProfileDto> {
    value.profile()
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

fn extract_cookie_from_body(cookie: Option<&str>) -> Vec<String> {
    cookie
        .unwrap_or_default()
        .split(";;")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn response_with_cookies<T>(
    client: &NeteaseClient,
    body: T,
    cookie: Option<&str>,
) -> ResponseWithCookies<T> {
    let mut set_cookie = client.take_last_set_cookie();
    set_cookie.extend(extract_cookie_from_body(cookie));
    ResponseWithCookies { body, set_cookie }
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
