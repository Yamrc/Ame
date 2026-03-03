use keyring::Entry;

use crate::error::{CoreError, Result};

const SERVICE_NAME: &str = "ame";
const COOKIE_KEY: &str = "netease_cookie";

#[derive(Clone, Default)]
pub struct CredentialStore;

impl CredentialStore {
    pub fn save_cookie(&self, cookie: &str) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, COOKIE_KEY)
            .map_err(|e| CoreError::Secure(e.to_string()))?;
        entry
            .set_password(cookie)
            .map_err(|e| CoreError::Secure(e.to_string()))?;
        Ok(())
    }

    pub fn load_cookie(&self) -> Result<Option<String>> {
        let entry = Entry::new(SERVICE_NAME, COOKIE_KEY)
            .map_err(|e| CoreError::Secure(e.to_string()))?;
        match entry.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CoreError::Secure(e.to_string())),
        }
    }

    pub fn delete_cookie(&self) -> Result<()> {
        let entry = Entry::new(SERVICE_NAME, COOKIE_KEY)
            .map_err(|e| CoreError::Secure(e.to_string()))?;
        match entry.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CoreError::Secure(e.to_string())),
        }
    }
}
