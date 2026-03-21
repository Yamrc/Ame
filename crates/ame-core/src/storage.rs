use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::{CoreError, Result};

const SETTINGS_DB_FILE_NAME: &str = "settings.redb";
const STATE_DB_FILE_NAME: &str = "state.redb";
const FIREWORK_DB_FILE_NAME: &str = "firework.redb";
const WEATHER_DB_FILE_NAME: &str = "weather.redb";
const GEOLOGICAL_DB_FILE_NAME: &str = "geological.redb";

const KV_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("kv");
const NETWORK_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("meta");
const NETWORK_INLINE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("inline");
const NETWORK_KEY_TAGS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("key_tags");
const NETWORK_TAG_INDEX_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tag_index");

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct AppStorage {
    settings: SettingsStorage,
    state: StateStorage,
    firework: NetworkCacheBucketStorage,
    weather: NetworkCacheBucketStorage,
    geological: NetworkCacheBucketStorage,
    response_dir: PathBuf,
}

impl AppStorage {
    pub fn open(base_dir: impl AsRef<Path>) -> Result<Self> {
        let paths = StoragePaths::from_root(base_dir.as_ref());
        paths.create_dirs()?;

        Ok(Self {
            settings: SettingsStorage::new(open_db_with_tables(
                &paths.settings_db,
                &[InitTableKind::Kv],
            )?),
            state: StateStorage::new(open_db_with_tables(&paths.state_db, &[InitTableKind::Kv])?),
            firework: NetworkCacheBucketStorage::new(
                open_db_with_tables(
                    &paths.firework_db,
                    &[
                        InitTableKind::NetworkMeta,
                        InitTableKind::NetworkInline,
                        InitTableKind::NetworkKeyTags,
                        InitTableKind::NetworkTagIndex,
                    ],
                )?,
                "firework",
            ),
            weather: NetworkCacheBucketStorage::new(
                open_db_with_tables(
                    &paths.weather_db,
                    &[
                        InitTableKind::NetworkMeta,
                        InitTableKind::NetworkInline,
                        InitTableKind::NetworkKeyTags,
                        InitTableKind::NetworkTagIndex,
                    ],
                )?,
                "weather",
            ),
            geological: NetworkCacheBucketStorage::new(
                open_db_with_tables(
                    &paths.geological_db,
                    &[
                        InitTableKind::NetworkMeta,
                        InitTableKind::NetworkInline,
                        InitTableKind::NetworkKeyTags,
                        InitTableKind::NetworkTagIndex,
                    ],
                )?,
                "geological",
            ),
            response_dir: paths.response_dir,
        })
    }

    pub fn temporary() -> Result<Self> {
        let base = std::env::temp_dir().join("ame-storage");
        std::fs::create_dir_all(&base).map_err(storage_err)?;
        let pid = std::process::id();
        let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_dir = base.join(format!("temp-{pid}-{seq}"));
        let _ = std::fs::remove_dir_all(&temp_dir);
        Self::open(temp_dir)
    }

    pub fn settings(&self) -> SettingsStorage {
        self.settings.clone()
    }

    pub fn state(&self) -> StateStorage {
        self.state.clone()
    }

    pub fn firework(&self) -> NetworkCacheBucketStorage {
        self.firework.clone()
    }

    pub fn weather(&self) -> NetworkCacheBucketStorage {
        self.weather.clone()
    }

    pub fn geological(&self) -> NetworkCacheBucketStorage {
        self.geological.clone()
    }

    pub fn response_dir(&self) -> &Path {
        &self.response_dir
    }
}

#[derive(Clone)]
pub struct SettingsStorage {
    db: Arc<Database>,
}

impl SettingsStorage {
    fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        set_json(&self.db, KV_TABLE, key, value)
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        get_json(&self.db, KV_TABLE, key)
    }
}

#[derive(Clone)]
pub struct StateStorage {
    db: Arc<Database>,
}

impl StateStorage {
    fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        set_json(&self.db, KV_TABLE, key, value)
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        get_json(&self.db, KV_TABLE, key)
    }

    pub fn remove(&self, key: &str) -> Result<()> {
        remove_value(&self.db, KV_TABLE, key)
    }
}

#[derive(Clone)]
pub struct NetworkCacheBucketStorage {
    db: Arc<Database>,
    bucket_name: &'static str,
}

impl NetworkCacheBucketStorage {
    fn new(db: Arc<Database>, bucket_name: &'static str) -> Self {
        Self { db, bucket_name }
    }

    pub fn bucket_name(&self) -> &'static str {
        self.bucket_name
    }

    pub fn set_meta<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        set_json(&self.db, NETWORK_META_TABLE, key, value)
    }

    pub fn get_meta<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        get_json(&self.db, NETWORK_META_TABLE, key)
    }

    pub fn iter_meta<T: DeserializeOwned>(&self) -> Result<Vec<(String, T)>> {
        iter_json(&self.db, NETWORK_META_TABLE)
    }

    pub fn set_inline_body(&self, key: &str, body: &[u8]) -> Result<()> {
        set_bytes(&self.db, NETWORK_INLINE_TABLE, key, body)
    }

    pub fn get_inline_body(&self, key: &str) -> Result<Option<Vec<u8>>> {
        get_bytes(&self.db, NETWORK_INLINE_TABLE, key)
    }

    pub fn replace_tags(&self, key: &str, tags: &[String]) -> Result<()> {
        let previous = self.get_key_tags(key)?;
        let write_txn = self.db.begin_write().map_err(storage_err)?;
        {
            let mut key_tags = write_txn
                .open_table(NETWORK_KEY_TAGS_TABLE)
                .map_err(storage_err)?;
            let mut tag_index = write_txn
                .open_table(NETWORK_TAG_INDEX_TABLE)
                .map_err(storage_err)?;

            for tag in previous {
                let composite = tag_index_key(&tag, key);
                let _ = tag_index.remove(composite.as_str()).map_err(storage_err)?;
            }

            if tags.is_empty() {
                let _ = key_tags.remove(key).map_err(storage_err)?;
            } else {
                let payload = serde_json::to_vec(tags)?;
                key_tags
                    .insert(key, payload.as_slice())
                    .map_err(storage_err)?;
                for tag in tags {
                    let composite = tag_index_key(tag, key);
                    tag_index
                        .insert(composite.as_str(), &[] as &[u8])
                        .map_err(storage_err)?;
                }
            }
        }
        write_txn.commit().map_err(storage_err)?;
        Ok(())
    }

    pub fn get_key_tags(&self, key: &str) -> Result<Vec<String>> {
        Ok(get_json(&self.db, NETWORK_KEY_TAGS_TABLE, key)?.unwrap_or_default())
    }

    pub fn keys_for_tag(&self, tag: &str) -> Result<Vec<String>> {
        let prefix = format!("{tag}\u{1f}");
        let read_txn = self.db.begin_read().map_err(storage_err)?;
        let table = read_txn
            .open_table(NETWORK_TAG_INDEX_TABLE)
            .map_err(storage_err)?;
        let iter = table.iter().map_err(storage_err)?;
        let mut keys = Vec::new();
        for entry in iter {
            let (raw_key, _) = entry.map_err(storage_err)?;
            let raw_key = raw_key.value();
            if raw_key.starts_with(&prefix) {
                keys.push(raw_key[prefix.len()..].to_string());
            }
        }
        Ok(keys)
    }

    pub fn remove(&self, key: &str) -> Result<()> {
        let tags = self.get_key_tags(key)?;
        let write_txn = self.db.begin_write().map_err(storage_err)?;
        {
            let mut meta = write_txn
                .open_table(NETWORK_META_TABLE)
                .map_err(storage_err)?;
            let mut inline = write_txn
                .open_table(NETWORK_INLINE_TABLE)
                .map_err(storage_err)?;
            let mut key_tags = write_txn
                .open_table(NETWORK_KEY_TAGS_TABLE)
                .map_err(storage_err)?;
            let mut tag_index = write_txn
                .open_table(NETWORK_TAG_INDEX_TABLE)
                .map_err(storage_err)?;

            let _ = meta.remove(key).map_err(storage_err)?;
            let _ = inline.remove(key).map_err(storage_err)?;
            let _ = key_tags.remove(key).map_err(storage_err)?;
            for tag in tags {
                let composite = tag_index_key(&tag, key);
                let _ = tag_index.remove(composite.as_str()).map_err(storage_err)?;
            }
        }
        write_txn.commit().map_err(storage_err)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct StoragePaths {
    settings_db: PathBuf,
    state_db: PathBuf,
    firework_db: PathBuf,
    weather_db: PathBuf,
    geological_db: PathBuf,
    response_dir: PathBuf,
}

impl StoragePaths {
    fn from_root(root: &Path) -> Self {
        let data_dir = root.join("data");
        let cache_dir = data_dir.join("cache");
        Self {
            settings_db: data_dir.join(SETTINGS_DB_FILE_NAME),
            state_db: data_dir.join(STATE_DB_FILE_NAME),
            firework_db: cache_dir.join(FIREWORK_DB_FILE_NAME),
            weather_db: cache_dir.join(WEATHER_DB_FILE_NAME),
            geological_db: cache_dir.join(GEOLOGICAL_DB_FILE_NAME),
            response_dir: cache_dir.join("response"),
        }
    }

    fn create_dirs(&self) -> Result<()> {
        for dir in [
            self.settings_db.parent(),
            self.firework_db.parent(),
            Some(self.response_dir.as_path()),
        ]
        .into_iter()
        .flatten()
        {
            std::fs::create_dir_all(dir).map_err(storage_err)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum InitTableKind {
    Kv,
    NetworkMeta,
    NetworkInline,
    NetworkKeyTags,
    NetworkTagIndex,
}

fn open_db_with_tables(path: &Path, tables: &[InitTableKind]) -> Result<Arc<Database>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(storage_err)?;
    }
    let db = Database::create(path).map_err(storage_err)?;
    let write_txn = db.begin_write().map_err(storage_err)?;
    {
        for table in tables {
            match table {
                InitTableKind::Kv => {
                    let _ = write_txn.open_table(KV_TABLE).map_err(storage_err)?;
                }
                InitTableKind::NetworkMeta => {
                    let _ = write_txn
                        .open_table(NETWORK_META_TABLE)
                        .map_err(storage_err)?;
                }
                InitTableKind::NetworkInline => {
                    let _ = write_txn
                        .open_table(NETWORK_INLINE_TABLE)
                        .map_err(storage_err)?;
                }
                InitTableKind::NetworkKeyTags => {
                    let _ = write_txn
                        .open_table(NETWORK_KEY_TAGS_TABLE)
                        .map_err(storage_err)?;
                }
                InitTableKind::NetworkTagIndex => {
                    let _ = write_txn
                        .open_table(NETWORK_TAG_INDEX_TABLE)
                        .map_err(storage_err)?;
                }
            }
        }
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
    set_bytes(db, table, key, payload.as_slice())
}

fn get_json<T: DeserializeOwned>(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
) -> Result<Option<T>> {
    let Some(payload) = get_bytes(db, table, key)? else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&payload)?))
}

fn iter_json<T: DeserializeOwned>(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
) -> Result<Vec<(String, T)>> {
    let read_txn = db.begin_read().map_err(storage_err)?;
    let bucket = read_txn.open_table(table).map_err(storage_err)?;
    let iter = bucket.iter().map_err(storage_err)?;
    let mut items = Vec::new();
    for entry in iter {
        let (key, value) = entry.map_err(storage_err)?;
        let payload: T = serde_json::from_slice(value.value())?;
        items.push((key.value().to_string(), payload));
    }
    Ok(items)
}

fn set_bytes(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
    value: &[u8],
) -> Result<()> {
    let write_txn = db.begin_write().map_err(storage_err)?;
    {
        let mut bucket = write_txn.open_table(table).map_err(storage_err)?;
        bucket.insert(key, value).map_err(storage_err)?;
    }
    write_txn.commit().map_err(storage_err)?;
    Ok(())
}

fn get_bytes(
    db: &Database,
    table: TableDefinition<&str, &[u8]>,
    key: &str,
) -> Result<Option<Vec<u8>>> {
    let read_txn = db.begin_read().map_err(storage_err)?;
    let bucket = read_txn.open_table(table).map_err(storage_err)?;
    let Some(raw) = bucket.get(key).map_err(storage_err)? else {
        return Ok(None);
    };
    Ok(Some(raw.value().to_vec()))
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

fn tag_index_key(tag: &str, key: &str) -> String {
    format!("{tag}\u{1f}{key}")
}

fn storage_err(err: impl std::fmt::Display) -> CoreError {
    CoreError::Storage(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::AppStorage;

    #[test]
    fn settings_round_trip() {
        let storage = AppStorage::temporary().expect("temp storage");
        let settings = storage.settings();
        settings.set("player.volume", &0.8_f32).expect("set volume");
        let value: Option<f32> = settings.get("player.volume").expect("get volume");
        assert_eq!(value, Some(0.8));
    }

    #[test]
    fn state_remove() {
        let storage = AppStorage::temporary().expect("temp storage");
        let state = storage.state();
        state.set("queue.current_index", &3_u32).expect("set index");
        state.remove("queue.current_index").expect("remove index");
        let value: Option<u32> = state.get("queue.current_index").expect("get index");
        assert_eq!(value, None);
    }

    #[test]
    fn network_bucket_tags_round_trip() {
        let storage = AppStorage::temporary().expect("temp storage");
        let bucket = storage.weather();
        bucket
            .set_meta("entry.1", &"ok".to_string())
            .expect("set meta");
        bucket
            .set_inline_body("entry.1", b"hello")
            .expect("set inline");
        bucket
            .replace_tags(
                "entry.1",
                &[String::from("search"), String::from("query:yoasobi")],
            )
            .expect("replace tags");

        let value: Option<String> = bucket.get_meta("entry.1").expect("get meta");
        assert_eq!(value.as_deref(), Some("ok"));
        assert_eq!(
            bucket.keys_for_tag("search").expect("keys for tag"),
            vec![String::from("entry.1")]
        );
        assert_eq!(
            bucket.get_inline_body("entry.1").expect("inline body"),
            Some(b"hello".to_vec())
        );
    }

    #[test]
    fn temporary_storage_has_cache_dirs() {
        let storage = AppStorage::temporary().expect("temp storage");
        assert!(storage.response_dir().exists());
    }
}
