use ame_core::credential::AuthBundle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct SessionState {
    pub auth_bundle: AuthBundle,
    pub auth_account_summary: Option<String>,
    pub auth_user_name: Option<String>,
    pub auth_user_avatar: Option<String>,
    pub auth_user_id: Option<i64>,
    pub guest_loading: bool,
    pub summary_loading: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistedSessionIdentity {
    pub token_fingerprint: String,
    pub user_id: i64,
    pub user_name: Option<String>,
    pub user_avatar: Option<String>,
    pub account_summary: Option<String>,
}

impl PersistedSessionIdentity {
    pub fn from_session(session: &SessionState) -> Option<Self> {
        let token_fingerprint = session_identity_fingerprint(&session.auth_bundle)?;
        let user_id = session.auth_user_id?;
        Some(Self {
            token_fingerprint,
            user_id,
            user_name: session.auth_user_name.clone(),
            user_avatar: session.auth_user_avatar.clone(),
            account_summary: session.auth_account_summary.clone(),
        })
    }

    pub fn matches_bundle(&self, bundle: &AuthBundle) -> bool {
        session_identity_fingerprint(bundle)
            .as_deref()
            .is_some_and(|fingerprint| fingerprint == self.token_fingerprint)
    }

    pub fn apply_to_session(&self, session: &mut SessionState) {
        session.auth_user_id = Some(self.user_id);
        session.auth_user_name = self.user_name.clone();
        session.auth_user_avatar = self.user_avatar.clone();
        session.auth_account_summary = self.account_summary.clone();
    }
}

pub fn session_identity_fingerprint(bundle: &AuthBundle) -> Option<String> {
    let music_u = bundle.music_u.as_deref()?.trim();
    if music_u.is_empty() {
        return None;
    }
    Some(blake3::hash(music_u.as_bytes()).to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use ame_core::credential::AuthBundle;

    use super::{PersistedSessionIdentity, SessionState, session_identity_fingerprint};

    #[test]
    fn fingerprint_requires_music_u() {
        assert_eq!(session_identity_fingerprint(&AuthBundle::default()), None);
        assert!(
            session_identity_fingerprint(&AuthBundle {
                music_u: Some("token".to_string()),
                music_a: None,
                csrf: None,
                music_r_t: None,
            })
            .is_some()
        );
    }

    #[test]
    fn persisted_identity_roundtrip_matches_bundle() {
        let bundle = AuthBundle {
            music_u: Some("token".to_string()),
            music_a: None,
            csrf: None,
            music_r_t: None,
        };
        let session = SessionState {
            auth_bundle: bundle.clone(),
            auth_account_summary: Some("summary".to_string()),
            auth_user_name: Some("name".to_string()),
            auth_user_avatar: Some("avatar".to_string()),
            auth_user_id: Some(42),
            guest_loading: false,
            summary_loading: false,
        };
        let identity = PersistedSessionIdentity::from_session(&session).expect("identity");
        assert!(identity.matches_bundle(&bundle));

        let mut restored = SessionState {
            auth_bundle: bundle,
            ..SessionState::default()
        };
        identity.apply_to_session(&mut restored);
        assert_eq!(restored.auth_user_id, Some(42));
        assert_eq!(restored.auth_user_name.as_deref(), Some("name"));
    }
}
