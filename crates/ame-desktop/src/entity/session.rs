use ame_core::credential::AuthBundle;

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub auth_bundle: AuthBundle,
    pub auth_account_summary: Option<String>,
    pub auth_user_name: Option<String>,
    pub auth_user_avatar: Option<String>,
    pub auth_user_id: Option<i64>,
}
