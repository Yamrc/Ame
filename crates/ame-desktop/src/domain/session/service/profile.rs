use ame_netease::api::common::models::UserProfileDto;
use ame_netease::api::user::status::LoginStatusResponse;

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
