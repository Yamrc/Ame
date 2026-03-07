use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use redb::{Database, TableDefinition};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{CoreError, Result};

const DB_FILE_NAME: &str = "app.redb";
const SETTINGS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("settings");
const STATE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("state");
const CACHE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("cache");
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct AppStorage {
    db: Arc<Database>,
}

impl AppStorage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db_path = normalize_db_path(path.as_ref());
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(storage_err)?;
        }
        let db = open_database_file(&db_path)?;
        Ok(Self { db })
    }

    pub fn temporary() -> Result<Self> {
        let base = std::env::temp_dir().join("ame-redb");
        std::fs::create_dir_all(&base).map_err(storage_err)?;
        let pid = std::process::id();
        let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let db_path = base.join(format!("temp-{pid}-{seq}.redb"));
        let db = open_database_file(&db_path)?;
        Ok(Self { db })
    }

    pub fn settings(&self) -> Result<SettingsStore> {
        Ok(SettingsStore::new(self.db.clone()))
    }

    pub fn state(&self) -> Result<StateStore> {
        Ok(StateStore::new(self.db.clone()))
    }

    pub fn cache(&self) -> Result<CacheIndexStore> {
        Ok(CacheIndexStore::new(self.db.clone()))
    }
}

#[derive(Clone)]
pub struct SettingsStore {
    db: Arc<Database>,
}

impl SettingsStore {
    fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        set_json(&self.db, SETTINGS_TABLE, key, value)
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        get_json(&self.db, SETTINGS_TABLE, key)
    }
}

#[derive(Clone)]
pub struct StateStore {
    db: Arc<Database>,
}

impl StateStore {
    fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        set_json(&self.db, STATE_TABLE, key, value)
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        get_json(&self.db, STATE_TABLE, key)
    }

    pub fn remove(&self, key: &str) -> Result<()> {
        remove_value(&self.db, STATE_TABLE, key)
    }
}

#[derive(Clone)]
pub struct CacheIndexStore {
    db: Arc<Database>,
}

impl CacheIndexStore {
    fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn upsert<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        set_json(&self.db, CACHE_TABLE, key, value)
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        get_json(&self.db, CACHE_TABLE, key)
    }
}

fn normalize_db_path(path: &Path) -> PathBuf {
    if path.extension().and_then(|ext| ext.to_str()) == Some("redb") {
        return path.to_path_buf();
    }
    path.join(DB_FILE_NAME)
}

fn open_database_file(path: &Path) -> Result<Arc<Database>> {
    let db = Database::create(path).map_err(storage_err)?;
    let write_txn = db.begin_write().map_err(storage_err)?;
    {
        let _ = write_txn.open_table(SETTINGS_TABLE).map_err(storage_err)?;
        let _ = write_txn.open_table(STATE_TABLE).map_err(storage_err)?;
        let _ = write_txn.open_table(CACHE_TABLE).map_err(storage_err)?;
    }
    write_txn.commit().map_err(storage_err)?;
    Ok(Arc::new(db))
}

fn set_json<T: Serialize>(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
    value: &T,
) -> Result<()> {
    let payload = serde_json::to_vec(value)?;
    let write_txn = db.begin_write().map_err(storage_err)?;
    {
        let mut bucket = write_txn.open_table(table).map_err(storage_err)?;
        bucket
            .insert(key, payload.as_slice())
            .map_err(storage_err)?;
    }
    write_txn.commit().map_err(storage_err)?;
    Ok(())
}

fn get_json<T: DeserializeOwned>(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
) -> Result<Option<T>> {
    let read_txn = db.begin_read().map_err(storage_err)?;
    let bucket = read_txn.open_table(table).map_err(storage_err)?;
    let Some(raw) = bucket.get(key).map_err(storage_err)? else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(raw.value())?))
}

fn remove_value(db: &Database, table: TableDefinition<&str, &[u8]>, key: &str) -> Result<()> {
    let write_txn = db.begin_write().map_err(storage_err)?;
    {
        let mut bucket = write_txn.open_table(table).map_err(storage_err)?;
        let _ = bucket.remove(key).map_err(storage_err)?;
    }
    write_txn.commit().map_err(storage_err)?;
    Ok(())
}

fn storage_err(err: impl std::fmt::Display) -> CoreError {
    CoreError::Storage(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::AppStorage;

    #[test]
    fn settings_round_trip() {
        let storage = AppStorage::temporary().expect("temp db");
        let settings = storage.settings().expect("settings tree");
        settings.set("player.volume", &0.8_f32).expect("set volume");
        let value: Option<f32> = settings.get("player.volume").expect("get volume");
        assert_eq!(value, Some(0.8));
    }

    #[test]
    fn state_remove() {
        let storage = AppStorage::temporary().expect("temp db");
        let state = storage.state().expect("state tree");
        state.set("queue.current_index", &3_u32).expect("set index");
        state.remove("queue.current_index").expect("remove index");
        let value: Option<u32> = state.get("queue.current_index").expect("get index");
        assert_eq!(value, None);
    }

    #[test]
    fn cache_upsert_get() {
        let storage = AppStorage::temporary().expect("temp db");
        let cache = storage.cache().expect("cache table");
        cache.upsert("cover.1", &"ok").expect("upsert cache");
        let value: Option<String> = cache.get("cover.1").expect("get cache");
        assert_eq!(value.as_deref(), Some("ok"));
    }

    #[test]
    fn temporary_storage_lifecycle() {
        let storage = AppStorage::temporary().expect("temp db");
        let settings = storage.settings().expect("settings table");
        settings.set("boot.marker", &true).expect("write marker");
        let value: Option<bool> = settings.get("boot.marker").expect("read marker");
        assert_eq!(value, Some(true));
    }
}
