use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use ame_core::storage::NetworkCacheBucketStorage;
use blake3::Hasher;
use dashmap::DashMap;
use parking_lot::Mutex;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::warn;

const INLINE_BODY_LIMIT: usize = 32 * 1024;
const ACCESS_BUCKET_MS: u64 = 5 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CacheClass {
    Firework,
    Weather,
    Geological,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CacheScope {
    Public,
    Guest,
    User(i64),
}

impl Display for CacheScope {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => f.write_str("public"),
            Self::Guest => f.write_str("guest"),
            Self::User(user_id) => write!(f, "user:{user_id}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheKey {
    namespace: String,
    version: u32,
    scope: CacheScope,
    canonical_params: String,
    digest: String,
}

impl CacheKey {
    pub fn new<T: Serialize>(
        namespace: impl Into<String>,
        version: u32,
        scope: CacheScope,
        params: &T,
    ) -> Result<Self, String> {
        let namespace = namespace.into();
        let canonical_params = serde_json::to_string(params)
            .map_err(|err| format!("Failed to serialize cache key params: {err}"))?;
        let logical_key = format!("{namespace}:{version}:{scope}:{canonical_params}");
        let mut hasher = Hasher::new();
        hasher.update(logical_key.as_bytes());
        let digest = hasher.finalize().to_hex().to_string();
        Ok(Self {
            namespace,
            version,
            scope,
            canonical_params,
            digest,
        })
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn scope(&self) -> &CacheScope {
        &self.scope
    }

    fn logical_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.namespace, self.version, self.scope, self.canonical_params
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CachePolicy {
    pub fresh_ttl_ms: u64,
    pub stale_ttl_ms: Option<u64>,
    pub revalidate_after_ms: u64,
    pub serve_stale_while_revalidate: bool,
    pub max_body_bytes: usize,
}

impl CachePolicy {
    pub const fn firework() -> Self {
        Self {
            fresh_ttl_ms: 30_000,
            stale_ttl_ms: None,
            revalidate_after_ms: 30_000,
            serve_stale_while_revalidate: true,
            max_body_bytes: 2 * 1024 * 1024,
        }
    }

    pub const fn weather() -> Self {
        Self {
            fresh_ttl_ms: 30 * 60 * 1000,
            stale_ttl_ms: Some(24 * 60 * 60 * 1000),
            revalidate_after_ms: 30 * 60 * 1000,
            serve_stale_while_revalidate: true,
            max_body_bytes: 4 * 1024 * 1024,
        }
    }

    pub const fn geological() -> Self {
        Self {
            fresh_ttl_ms: 30 * 24 * 60 * 60 * 1000,
            stale_ttl_ms: None,
            revalidate_after_ms: 7 * 24 * 60 * 60 * 1000,
            serve_stale_while_revalidate: true,
            max_body_bytes: 4 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheValue<T> {
    pub value: T,
    pub fetched_at_ms: u64,
}

#[derive(Debug, Clone)]
pub enum CacheLookup<T> {
    Miss,
    Fresh(CacheValue<T>),
    Stale(CacheValue<T>),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum CacheBodyRef {
    Inline,
    Blob { relative_path: String },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CacheMeta {
    class: CacheClass,
    scope: CacheScope,
    logical_key: String,
    tags: Vec<String>,
    fetched_at_ms: u64,
    last_validated_at_ms: u64,
    last_accessed_bucket: u64,
    expires_at_ms: u64,
    stale_until_ms: Option<u64>,
    error_backoff_until_ms: Option<u64>,
    size_bytes: u64,
    payload_hash: String,
    body_ref: CacheBodyRef,
}

#[derive(Clone)]
pub struct CacheService {
    firework: NetworkCacheBucketStorage,
    weather: NetworkCacheBucketStorage,
    geological: NetworkCacheBucketStorage,
    response_dir: PathBuf,
    inflight: Arc<DashMap<String, Arc<Mutex<()>>>>,
}

impl CacheService {
    pub fn new(
        firework: NetworkCacheBucketStorage,
        weather: NetworkCacheBucketStorage,
        geological: NetworkCacheBucketStorage,
        response_dir: PathBuf,
    ) -> Self {
        Self {
            firework,
            weather,
            geological,
            response_dir,
            inflight: Arc::new(DashMap::new()),
        }
    }

    pub fn read_json<T: DeserializeOwned>(
        &self,
        class: CacheClass,
        key: &CacheKey,
        policy: CachePolicy,
    ) -> Result<CacheLookup<T>, String> {
        let bucket = self.bucket(class);
        let Some(mut meta) = bucket
            .get_meta::<CacheMeta>(key.digest())
            .map_err(|err| format!("Failed to read cache metadata: {err}"))?
        else {
            return Ok(CacheLookup::Miss);
        };

        let body = match self.read_body(bucket, key.digest(), &meta) {
            Ok(body) => body,
            Err(err) => {
                warn!(
                    cache_key = key.logical_key(),
                    bucket = bucket.bucket_name(),
                    error = %err,
                    "cache body read failed"
                );
                return Ok(CacheLookup::Miss);
            }
        };

        self.touch_access_bucket(bucket, key.digest(), &mut meta)
            .map_err(|err| format!("Failed to update cache access time: {err}"))?;

        let value = serde_json::from_slice::<T>(&body)
            .map_err(|err| format!("Failed to parse cache payload: {err}"))?;
        let now_ms = now_millis();
        let fresh = now_ms <= meta.expires_at_ms;
        let revalidate_due = now_ms
            >= meta
                .last_validated_at_ms
                .saturating_add(policy.revalidate_after_ms);
        let always_revalidate = matches!(class, CacheClass::Firework);
        let serve_stale = policy.serve_stale_while_revalidate
            && meta
                .stale_until_ms
                .is_none_or(|stale_until_ms| now_ms <= stale_until_ms);

        if fresh && !revalidate_due && !always_revalidate {
            return Ok(CacheLookup::Fresh(CacheValue {
                value,
                fetched_at_ms: meta.fetched_at_ms,
            }));
        }

        if serve_stale {
            return Ok(CacheLookup::Stale(CacheValue {
                value,
                fetched_at_ms: meta.fetched_at_ms,
            }));
        }

        Ok(CacheLookup::Miss)
    }

    pub fn fetch_and_store_json<T, F>(
        &self,
        class: CacheClass,
        key: &CacheKey,
        policy: CachePolicy,
        tags: &[String],
        fetcher: F,
    ) -> Result<CacheValue<T>, String>
    where
        T: Serialize + DeserializeOwned + Clone,
        F: FnOnce() -> Result<T, String>,
    {
        let lock = self
            .inflight
            .entry(key.digest().to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();
        let _guard = lock.lock();

        if let CacheLookup::Fresh(value) = self.read_json(class, key, policy)? {
            return Ok(value);
        }

        let value = fetcher()?;
        let fetched_at_ms = self.write_json(class, key, policy, tags, &value)?;
        Ok(CacheValue {
            value,
            fetched_at_ms,
        })
    }

    pub fn write_json<T: Serialize + ?Sized>(
        &self,
        class: CacheClass,
        key: &CacheKey,
        policy: CachePolicy,
        tags: &[String],
        value: &T,
    ) -> Result<u64, String> {
        let payload = serde_json::to_vec(value)
            .map_err(|err| format!("Failed to serialize cache payload: {err}"))?;
        if payload.len() > policy.max_body_bytes {
            return Err(format!(
                "Cache payload exceeds size limit: {} > {}",
                payload.len(),
                policy.max_body_bytes
            ));
        }

        let bucket = self.bucket(class);
        let previous_meta = bucket
            .get_meta::<CacheMeta>(key.digest())
            .map_err(|err| format!("Failed to read previous cache metadata: {err}"))?;
        let fetched_at_ms = now_millis();
        let payload_hash = blake3::hash(&payload).to_hex().to_string();
        let body_ref = if payload.len() <= INLINE_BODY_LIMIT {
            bucket
                .set_inline_body(key.digest(), &payload)
                .map_err(|err| format!("Failed to write inline cache payload: {err}"))?;
            CacheBodyRef::Inline
        } else {
            let relative_path = response_blob_relative_path(class, key.digest());
            let full_path = self.response_dir.join(&relative_path);
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("Failed to create cache directory: {err}"))?;
            }
            fs::write(&full_path, &payload)
                .map_err(|err| format!("Failed to write cache file: {err}"))?;
            CacheBodyRef::Blob { relative_path }
        };

        let meta = CacheMeta {
            class,
            scope: key.scope().clone(),
            logical_key: key.logical_key(),
            tags: tags.to_vec(),
            fetched_at_ms,
            last_validated_at_ms: fetched_at_ms,
            last_accessed_bucket: access_bucket(fetched_at_ms),
            expires_at_ms: fetched_at_ms.saturating_add(policy.fresh_ttl_ms),
            stale_until_ms: policy
                .stale_ttl_ms
                .map(|stale_ttl_ms| fetched_at_ms.saturating_add(stale_ttl_ms)),
            error_backoff_until_ms: None,
            size_bytes: payload.len() as u64,
            payload_hash,
            body_ref,
        };
        bucket
            .set_meta(key.digest(), &meta)
            .map_err(|err| format!("Failed to write cache metadata: {err}"))?;
        bucket
            .replace_tags(key.digest(), tags)
            .map_err(|err| format!("Failed to write cache tag index: {err}"))?;
        self.remove_stale_body_file(previous_meta.as_ref(), &meta.body_ref)?;
        self.trim_class_if_needed(class)?;
        Ok(fetched_at_ms)
    }

    #[allow(dead_code)]
    pub fn invalidate_tag(&self, class: CacheClass, tag: &str) -> Result<(), String> {
        let bucket = self.bucket(class);
        let keys = bucket
            .keys_for_tag(tag)
            .map_err(|err| format!("Failed to read tag index: {err}"))?;
        for key in keys {
            self.remove_entry(class, &key)?;
        }
        Ok(())
    }

    pub fn invalidate_scope(&self, class: CacheClass, scope: &CacheScope) -> Result<(), String> {
        let bucket = self.bucket(class);
        let items = bucket
            .iter_meta::<CacheMeta>()
            .map_err(|err| format!("Failed to iterate cache metadata: {err}"))?;
        for (key, meta) in items {
            if &meta.scope == scope {
                self.remove_entry(class, &key)?;
            }
        }
        Ok(())
    }

    pub fn run_maintenance(&self) -> Result<(), String> {
        self.prune_orphan_response_files(CacheClass::Firework)?;
        self.prune_orphan_response_files(CacheClass::Weather)?;
        self.prune_orphan_response_files(CacheClass::Geological)?;
        self.trim_class_if_needed(CacheClass::Firework)?;
        self.trim_class_if_needed(CacheClass::Weather)?;
        self.trim_class_if_needed(CacheClass::Geological)?;
        Ok(())
    }

    fn trim_class_if_needed(&self, class: CacheClass) -> Result<(), String> {
        let bucket = self.bucket(class);
        let mut items = bucket
            .iter_meta::<CacheMeta>()
            .map_err(|err| format!("Failed to iterate cache metadata: {err}"))?;
        let budget_bytes = class_budget_bytes(class);
        let total_bytes = items.iter().map(|(_, meta)| meta.size_bytes).sum::<u64>() as usize;
        if total_bytes <= budget_bytes {
            return Ok(());
        }

        items.sort_by_key(|(_, meta)| {
            (
                meta.stale_until_ms
                    .is_none_or(|stale_until_ms| now_millis() <= stale_until_ms),
                meta.last_accessed_bucket,
            )
        });

        let mut reclaim_bytes = total_bytes.saturating_sub(budget_bytes);
        for (key, meta) in items {
            if reclaim_bytes == 0 {
                break;
            }
            self.remove_entry(class, &key)?;
            reclaim_bytes = reclaim_bytes.saturating_sub(meta.size_bytes as usize);
        }
        Ok(())
    }

    fn remove_entry(&self, class: CacheClass, key: &str) -> Result<(), String> {
        let bucket = self.bucket(class);
        let meta = bucket
            .get_meta::<CacheMeta>(key)
            .map_err(|err| format!("Failed to read cache metadata: {err}"))?;
        if let Some(meta) = meta
            && let CacheBodyRef::Blob { relative_path } = meta.body_ref
        {
            let full_path = self.response_dir.join(relative_path);
            match fs::remove_file(&full_path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(format!("Failed to delete cache file: {err}"));
                }
            }
        }
        bucket
            .remove(key)
            .map_err(|err| format!("Failed to delete cache entry: {err}"))?;
        Ok(())
    }

    fn remove_stale_body_file(
        &self,
        previous_meta: Option<&CacheMeta>,
        next_body_ref: &CacheBodyRef,
    ) -> Result<(), String> {
        let Some(previous_meta) = previous_meta else {
            return Ok(());
        };
        let CacheBodyRef::Blob {
            relative_path: previous_relative_path,
        } = &previous_meta.body_ref
        else {
            return Ok(());
        };
        let should_keep_previous = match next_body_ref {
            CacheBodyRef::Inline => false,
            CacheBodyRef::Blob { relative_path } => relative_path == previous_relative_path,
        };
        if should_keep_previous {
            return Ok(());
        }
        self.remove_blob_file(self.response_dir.join(previous_relative_path))
    }

    fn read_body(
        &self,
        bucket: &NetworkCacheBucketStorage,
        key: &str,
        meta: &CacheMeta,
    ) -> Result<Vec<u8>, String> {
        match &meta.body_ref {
            CacheBodyRef::Inline => bucket
                .get_inline_body(key)
                .map_err(|err| format!("Failed to read inline cache payload: {err}"))?
                .ok_or_else(|| "missing inline cache body".to_string()),
            CacheBodyRef::Blob { relative_path } => {
                let full_path = self.response_dir.join(relative_path);
                fs::read(full_path).map_err(|err| format!("Failed to read cache file: {err}"))
            }
        }
    }

    fn touch_access_bucket(
        &self,
        bucket: &NetworkCacheBucketStorage,
        key: &str,
        meta: &mut CacheMeta,
    ) -> Result<(), ame_core::error::CoreError> {
        let current_bucket = access_bucket(now_millis());
        if meta.last_accessed_bucket == current_bucket {
            return Ok(());
        }
        meta.last_accessed_bucket = current_bucket;
        bucket.set_meta(key, meta)
    }

    fn bucket(&self, class: CacheClass) -> &NetworkCacheBucketStorage {
        match class {
            CacheClass::Firework => &self.firework,
            CacheClass::Weather => &self.weather,
            CacheClass::Geological => &self.geological,
        }
    }

    fn prune_orphan_response_files(&self, class: CacheClass) -> Result<(), String> {
        let bucket = self.bucket(class);
        let items = bucket
            .iter_meta::<CacheMeta>()
            .map_err(|err| format!("Failed to iterate cache metadata: {err}"))?;
        let live_paths = items
            .into_iter()
            .filter_map(|(_, meta)| match meta.body_ref {
                CacheBodyRef::Inline => None,
                CacheBodyRef::Blob { relative_path } => Some(relative_path),
            })
            .collect::<std::collections::HashSet<_>>();
        let bucket_dir = self.response_dir.join(bucket.bucket_name());
        if !bucket_dir.exists() {
            return Ok(());
        }
        for file_path in collect_bucket_blob_files(&bucket_dir)? {
            let relative_path = file_path
                .strip_prefix(&self.response_dir)
                .map_err(|err| format!("Failed to parse cache relative path: {err}"))?
                .to_string_lossy()
                .replace('\\', "/");
            if !live_paths.contains(&relative_path) {
                self.remove_blob_file(file_path)?;
            }
        }
        Ok(())
    }

    fn remove_blob_file(&self, file_path: PathBuf) -> Result<(), String> {
        match fs::remove_file(&file_path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("Failed to delete cache file: {err}")),
        }
        cleanup_empty_parents(&file_path, &self.response_dir)
    }
}

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn access_bucket(timestamp_ms: u64) -> u64 {
    timestamp_ms / ACCESS_BUCKET_MS
}

fn class_budget_bytes(class: CacheClass) -> usize {
    match class {
        CacheClass::Firework => 32 * 1024 * 1024,
        CacheClass::Weather => 128 * 1024 * 1024,
        CacheClass::Geological => 256 * 1024 * 1024,
    }
}

fn response_blob_relative_path(class: CacheClass, digest: &str) -> String {
    let bucket = match class {
        CacheClass::Firework => "firework",
        CacheClass::Weather => "weather",
        CacheClass::Geological => "geological",
    };
    let aa = &digest[..2];
    let bb = &digest[2..4];
    format!("{bucket}/{aa}/{bb}/{digest}")
}

fn collect_bucket_blob_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries =
            fs::read_dir(&dir).map_err(|err| format!("Failed to read cache directory: {err}"))?;
        for entry in entries {
            let entry =
                entry.map_err(|err| format!("Failed to read cache directory entry: {err}"))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|err| format!("Failed to read cache file type: {err}"))?;
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn cleanup_empty_parents(path: &Path, stop_at: &Path) -> Result<(), String> {
    let mut current = path.parent();
    while let Some(dir) = current {
        if dir == stop_at {
            break;
        }
        match fs::remove_dir(dir) {
            Ok(()) => current = dir.parent(),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => current = dir.parent(),
            Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => break,
            Err(err) => return Err(format!("Failed to remove empty cache directory: {err}")),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ame_core::storage::AppStorage;
    use serde::{Deserialize, Serialize};

    use super::{
        CacheClass, CacheKey, CacheLookup, CachePolicy, CacheScope, CacheService,
        response_blob_relative_path,
    };

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestPayload {
        value: String,
    }

    fn test_service() -> CacheService {
        let storage = AppStorage::temporary().expect("temporary storage");
        CacheService::new(
            storage.firework(),
            storage.weather(),
            storage.geological(),
            storage.response_dir().to_path_buf(),
        )
    }

    #[test]
    fn cache_key_changes_with_scope() {
        let public = CacheKey::new("test", 1, CacheScope::Public, &("a", 1)).expect("public key");
        let user = CacheKey::new("test", 1, CacheScope::User(1), &("a", 1)).expect("user key");
        assert_ne!(public.digest(), user.digest());
    }

    #[test]
    fn write_and_read_inline_json() {
        let service = test_service();
        let key = CacheKey::new("inline", 1, CacheScope::Public, &()).expect("cache key");
        let payload = TestPayload {
            value: "hello".to_string(),
        };

        let fetched_at_ms = service
            .write_json(
                CacheClass::Firework,
                &key,
                CachePolicy::firework(),
                &["tag:inline".to_string()],
                &payload,
            )
            .expect("write inline");

        match service
            .read_json::<TestPayload>(CacheClass::Firework, &key, CachePolicy::firework())
            .expect("read inline")
        {
            CacheLookup::Stale(value) => {
                assert_eq!(value.value, payload);
                assert_eq!(value.fetched_at_ms, fetched_at_ms);
            }
            _ => panic!("expected stale inline cache"),
        }
    }

    #[test]
    fn write_and_read_blob_json() {
        let service = test_service();
        let key = CacheKey::new("blob", 1, CacheScope::Public, &()).expect("cache key");
        let payload = TestPayload {
            value: "x".repeat(40 * 1024),
        };

        service
            .write_json(
                CacheClass::Weather,
                &key,
                CachePolicy::weather(),
                &["tag:blob".to_string()],
                &payload,
            )
            .expect("write blob");

        let path = service.response_dir.join(response_blob_relative_path(
            CacheClass::Weather,
            key.digest(),
        ));
        assert!(path.exists());

        match service
            .read_json::<TestPayload>(CacheClass::Weather, &key, CachePolicy::weather())
            .expect("read blob")
        {
            CacheLookup::Fresh(value) => assert_eq!(value.value, payload),
            _ => panic!("expected fresh blob cache"),
        }
    }

    #[test]
    fn expired_entry_becomes_stale_when_policy_allows() {
        let service = test_service();
        let key = CacheKey::new("stale", 1, CacheScope::Public, &()).expect("cache key");
        let payload = TestPayload {
            value: "stale".to_string(),
        };

        service
            .write_json(
                CacheClass::Weather,
                &key,
                CachePolicy {
                    fresh_ttl_ms: 0,
                    stale_ttl_ms: Some(60_000),
                    revalidate_after_ms: 0,
                    serve_stale_while_revalidate: true,
                    max_body_bytes: 1024 * 1024,
                },
                &[],
                &payload,
            )
            .expect("write stale-capable");

        match service
            .read_json::<TestPayload>(
                CacheClass::Weather,
                &key,
                CachePolicy {
                    fresh_ttl_ms: 0,
                    stale_ttl_ms: Some(60_000),
                    revalidate_after_ms: 0,
                    serve_stale_while_revalidate: true,
                    max_body_bytes: 1024 * 1024,
                },
            )
            .expect("read stale-capable")
        {
            CacheLookup::Stale(value) => assert_eq!(value.value, payload),
            _ => panic!("expected stale cache"),
        }
    }

    #[test]
    fn firework_hit_is_served_as_stale_for_revalidation() {
        let service = test_service();
        let key = CacheKey::new("firework", 1, CacheScope::User(7), &()).expect("cache key");
        let payload = TestPayload {
            value: "firework".to_string(),
        };

        service
            .write_json(
                CacheClass::Firework,
                &key,
                CachePolicy::firework(),
                &[],
                &payload,
            )
            .expect("write firework");

        match service
            .read_json::<TestPayload>(CacheClass::Firework, &key, CachePolicy::firework())
            .expect("read firework")
        {
            CacheLookup::Stale(value) => assert_eq!(value.value, payload),
            _ => panic!("expected stale cache for firework hit"),
        }
    }

    #[test]
    fn firework_entry_without_stale_timeout_remains_servable() {
        let service = test_service();
        let key =
            CacheKey::new("firework-no-timeout", 1, CacheScope::User(8), &()).expect("cache key");
        let payload = TestPayload {
            value: "persist".to_string(),
        };

        service
            .write_json(
                CacheClass::Firework,
                &key,
                CachePolicy {
                    fresh_ttl_ms: 0,
                    stale_ttl_ms: None,
                    revalidate_after_ms: 0,
                    serve_stale_while_revalidate: true,
                    max_body_bytes: 1024 * 1024,
                },
                &[],
                &payload,
            )
            .expect("write firework without stale timeout");

        match service
            .read_json::<TestPayload>(
                CacheClass::Firework,
                &key,
                CachePolicy {
                    fresh_ttl_ms: 0,
                    stale_ttl_ms: None,
                    revalidate_after_ms: 0,
                    serve_stale_while_revalidate: true,
                    max_body_bytes: 1024 * 1024,
                },
            )
            .expect("read firework without stale timeout")
        {
            CacheLookup::Stale(value) => assert_eq!(value.value, payload),
            _ => panic!("expected servable stale cache for firework entry"),
        }
    }

    #[test]
    fn fresh_entry_becomes_stale_when_revalidate_is_due() {
        let service = test_service();
        let key = CacheKey::new("revalidate", 1, CacheScope::Public, &()).expect("cache key");
        let payload = TestPayload {
            value: "due".to_string(),
        };

        service
            .write_json(
                CacheClass::Weather,
                &key,
                CachePolicy {
                    fresh_ttl_ms: 60_000,
                    stale_ttl_ms: Some(60_000),
                    revalidate_after_ms: 0,
                    serve_stale_while_revalidate: true,
                    max_body_bytes: 1024 * 1024,
                },
                &[],
                &payload,
            )
            .expect("write revalidate");

        match service
            .read_json::<TestPayload>(
                CacheClass::Weather,
                &key,
                CachePolicy {
                    fresh_ttl_ms: 60_000,
                    stale_ttl_ms: Some(60_000),
                    revalidate_after_ms: 0,
                    serve_stale_while_revalidate: true,
                    max_body_bytes: 1024 * 1024,
                },
            )
            .expect("read revalidate")
        {
            CacheLookup::Stale(value) => assert_eq!(value.value, payload),
            _ => panic!("expected stale cache when revalidate is due"),
        }
    }

    #[test]
    fn invalidate_tag_removes_entries() {
        let service = test_service();
        let key = CacheKey::new("invalidate", 1, CacheScope::Public, &()).expect("cache key");
        let payload = TestPayload {
            value: "gone".to_string(),
        };

        service
            .write_json(
                CacheClass::Firework,
                &key,
                CachePolicy::firework(),
                &["tag:invalidate".to_string()],
                &payload,
            )
            .expect("write invalidate");
        service
            .invalidate_tag(CacheClass::Firework, "tag:invalidate")
            .expect("invalidate tag");

        assert!(matches!(
            service
                .read_json::<TestPayload>(CacheClass::Firework, &key, CachePolicy::firework())
                .expect("read invalidated"),
            CacheLookup::Miss
        ));
    }

    #[test]
    fn rewriting_blob_entry_to_inline_removes_old_blob_file() {
        let service = test_service();
        let key = CacheKey::new("rewrite", 1, CacheScope::Public, &()).expect("cache key");
        let large = TestPayload {
            value: "x".repeat(40 * 1024),
        };
        let small = TestPayload {
            value: "small".to_string(),
        };

        service
            .write_json(
                CacheClass::Weather,
                &key,
                CachePolicy::weather(),
                &[],
                &large,
            )
            .expect("write blob");
        let blob_path = service.response_dir.join(response_blob_relative_path(
            CacheClass::Weather,
            key.digest(),
        ));
        assert!(blob_path.exists());

        service
            .write_json(
                CacheClass::Weather,
                &key,
                CachePolicy::weather(),
                &[],
                &small,
            )
            .expect("rewrite inline");

        assert!(!blob_path.exists());
    }

    #[test]
    fn maintenance_removes_orphan_blob_files() {
        let service = test_service();
        let orphan_path = service.response_dir.join(response_blob_relative_path(
            CacheClass::Geological,
            "abcd1234abcd1234",
        ));
        std::fs::create_dir_all(orphan_path.parent().expect("orphan parent"))
            .expect("create orphan parent");
        std::fs::write(&orphan_path, b"orphan").expect("write orphan blob");
        assert!(orphan_path.exists());

        service.run_maintenance().expect("run maintenance");

        assert!(!orphan_path.exists());
    }
}
