use keyring::Entry;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};

const SERVICE_NAME: &str = "ame";
const AUTH_MUSIC_U_KEY: &str = "netease_auth_music_u";
const AUTH_MUSIC_A_KEY: &str = "netease_auth_music_a";
const AUTH_CSRF_KEY: &str = "netease_auth_csrf";
const AUTH_MUSIC_R_T_KEY: &str = "netease_auth_music_r_t";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthBundle {
    pub music_u: Option<String>,
    pub music_a: Option<String>,
    pub csrf: Option<String>,
    pub music_r_t: Option<String>,
}

#[derive(Clone, Default)]
pub struct CredentialStore;

impl CredentialStore {
    pub fn save_auth_bundle(&self, bundle: &AuthBundle) -> Result<()> {
        self.save_optional_secret(AUTH_MUSIC_U_KEY, bundle.music_u.as_deref())?;
        self.save_optional_secret(AUTH_MUSIC_A_KEY, bundle.music_a.as_deref())?;
        self.save_optional_secret(AUTH_CSRF_KEY, bundle.csrf.as_deref())?;
        self.save_optional_secret(AUTH_MUSIC_R_T_KEY, bundle.music_r_t.as_deref())?;
        Ok(())
    }

    pub fn load_auth_bundle(&self) -> Result<Option<AuthBundle>> {
        let bundle = AuthBundle {
            music_u: self.load_secret(AUTH_MUSIC_U_KEY)?,
            music_a: self.load_secret(AUTH_MUSIC_A_KEY)?,
            csrf: self.load_secret(AUTH_CSRF_KEY)?,
            music_r_t: self.load_secret(AUTH_MUSIC_R_T_KEY)?,
        };

        if has_any_auth_field(&bundle) {
            return Ok(Some(bundle));
        }
        Ok(None)
    }

    pub fn delete_auth_bundle(&self) -> Result<()> {
        self.delete_secret(AUTH_MUSIC_U_KEY)?;
        self.delete_secret(AUTH_MUSIC_A_KEY)?;
        self.delete_secret(AUTH_CSRF_KEY)?;
        self.delete_secret(AUTH_MUSIC_R_T_KEY)?;
        Ok(())
    }

    fn save_optional_secret(&self, key: &str, value: Option<&str>) -> Result<()> {
        match value.filter(|it| !it.trim().is_empty()) {
            Some(secret) => self.save_secret(key, secret),
            None => self.delete_secret(key),
        }
    }

    fn save_secret(&self, key: &str, secret: &str) -> Result<()> {
        self.entry(key)?
            .set_password(secret)
            .map_err(|e| CoreError::Secure(e.to_string()))?;
        Ok(())
    }

    fn load_secret(&self, key: &str) -> Result<Option<String>> {
        match self.entry(key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CoreError::Secure(e.to_string())),
        }
    }

    fn delete_secret(&self, key: &str) -> Result<()> {
        match self.entry(key)?.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CoreError::Secure(e.to_string())),
        }
    }

    fn entry(&self, key: &str) -> Result<Entry> {
        Entry::new(SERVICE_NAME, key).map_err(|e| CoreError::Secure(e.to_string()))
    }
}

fn has_any_auth_field(bundle: &AuthBundle) -> bool {
    bundle.music_u.is_some()
        || bundle.music_a.is_some()
        || bundle.csrf.is_some()
        || bundle.music_r_t.is_some()
}

#[cfg(test)]
mod tests {
    use super::{AuthBundle, has_any_auth_field};

    #[test]
    fn auth_bundle_json_roundtrip() {
        let bundle = AuthBundle {
            music_u: Some("u".to_string()),
            music_a: Some("a".to_string()),
            csrf: Some("c".to_string()),
            music_r_t: Some("r".to_string()),
        };
        let raw = serde_json::to_string(&bundle).expect("serialize auth bundle");
        let decoded: AuthBundle = serde_json::from_str(&raw).expect("deserialize auth bundle");
        assert_eq!(decoded, bundle);
    }

    #[test]
    fn auth_presence_detects_any_populated_field() {
        assert!(!has_any_auth_field(&AuthBundle::default()));
        assert!(has_any_auth_field(&AuthBundle {
            music_u: Some("u".to_string()),
            music_a: None,
            csrf: None,
            music_r_t: None,
        }));
    }
}
