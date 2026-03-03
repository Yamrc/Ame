use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::Result;

#[derive(Clone)]
pub struct AppStorage {
    db: sled::Db,
}

impl AppStorage {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn temporary() -> Result<Self> {
        let db = sled::Config::new().temporary(true).open()?;
        Ok(Self { db })
    }

    pub fn settings(&self) -> Result<SettingsStore> {
        Ok(SettingsStore::new(self.db.open_tree("settings")?))
    }

    pub fn state(&self) -> Result<StateStore> {
        Ok(StateStore::new(self.db.open_tree("state")?))
    }

    pub fn cache(&self) -> Result<CacheIndexStore> {
        Ok(CacheIndexStore::new(self.db.open_tree("cache")?))
    }
}

#[derive(Clone)]
pub struct SettingsStore {
    tree: sled::Tree,
}

impl SettingsStore {
    fn new(tree: sled::Tree) -> Self {
        Self { tree }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let payload = serde_json::to_vec(value)?;
        self.tree.insert(key.as_bytes(), payload)?;
        self.tree.flush()?;
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let Some(raw) = self.tree.get(key.as_bytes())? else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw)?))
    }
}

#[derive(Clone)]
pub struct StateStore {
    tree: sled::Tree,
}

impl StateStore {
    fn new(tree: sled::Tree) -> Self {
        Self { tree }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let payload = serde_json::to_vec(value)?;
        self.tree.insert(key.as_bytes(), payload)?;
        self.tree.flush()?;
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let Some(raw) = self.tree.get(key.as_bytes())? else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw)?))
    }

    pub fn remove(&self, key: &str) -> Result<()> {
        self.tree.remove(key.as_bytes())?;
        self.tree.flush()?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct CacheIndexStore {
    tree: sled::Tree,
}

impl CacheIndexStore {
    fn new(tree: sled::Tree) -> Self {
        Self { tree }
    }

    pub fn upsert<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let payload = serde_json::to_vec(value)?;
        self.tree.insert(key.as_bytes(), payload)?;
        self.tree.flush()?;
        Ok(())
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let Some(raw) = self.tree.get(key.as_bytes())? else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw)?))
    }
}

#[cfg(test)]
mod tests {
    use super::AppStorage;

    #[test]
    fn settings_round_trip() {
        let storage = AppStorage::temporary().expect("temp db");
        let settings = storage.settings().expect("settings tree");
        settings
            .set("player.volume", &0.8_f32)
            .expect("set volume");
        let value: Option<f32> = settings.get("player.volume").expect("get volume");
        assert_eq!(value, Some(0.8));
    }

    #[test]
    fn state_remove() {
        let storage = AppStorage::temporary().expect("temp db");
        let state = storage.state().expect("state tree");
        state
            .set("queue.current_index", &3_u32)
            .expect("set index");
        state.remove("queue.current_index").expect("remove index");
        let value: Option<u32> = state.get("queue.current_index").expect("get index");
        assert_eq!(value, None);
    }
}
