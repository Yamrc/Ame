use ame_core::secure::CredentialStore;
use ame_core::storage::AppStorage;

#[derive(Clone)]
pub struct InfraEntity {
    pub storage: AppStorage,
    pub credential: CredentialStore,
}

impl InfraEntity {
    pub fn temporary() -> ame_core::error::Result<Self> {
        Ok(Self {
            storage: AppStorage::temporary()?,
            credential: CredentialStore,
        })
    }
}
