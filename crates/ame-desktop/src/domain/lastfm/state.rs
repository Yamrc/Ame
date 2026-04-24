use serde::{Deserialize, Serialize};

use crate::domain::player::QueueItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastFmBuildConfig {
    pub api_key: Option<&'static str>,
    pub shared_secret: Option<&'static str>,
}

impl LastFmBuildConfig {
    pub const fn from_env() -> Self {
        Self {
            api_key: option_env!("AME_LASTFM_API_KEY"),
            shared_secret: option_env!("AME_LASTFM_SHARED_SECRET"),
        }
    }

    pub const fn is_configured(self) -> bool {
        matches!(
            (self.api_key, self.shared_secret),
            (Some(api_key), Some(shared_secret))
                if !api_key.is_empty() && !shared_secret.is_empty()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastFmSession {
    pub session_key: String,
    pub user_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastFmScrobbleRecord {
    pub track_id: i64,
    pub artist: String,
    pub track: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub started_at_unix_secs: u64,
    pub retry_count: u32,
    pub next_retry_at_ms: u64,
}

impl LastFmScrobbleRecord {
    pub fn matches_identity(&self, other: &Self) -> bool {
        self.track_id == other.track_id
            && self.started_at_unix_secs == other.started_at_unix_secs
            && self.artist == other.artist
            && self.track == other.track
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastFmCurrentPlayback {
    pub track_id: i64,
    pub artist: String,
    pub track: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub started_at_unix_secs: u64,
    pub played_ms: u64,
    pub last_tick_at_ms: Option<u64>,
    pub scrobbled: bool,
    pub now_playing_sent: bool,
}

impl LastFmCurrentPlayback {
    pub fn from_queue_item(item: &QueueItem, now_ms: u64, now_unix_secs: u64) -> Option<Self> {
        let artist = item.artist.trim();
        let track = item.name.trim();
        if artist.is_empty() || track.is_empty() {
            return None;
        }

        Some(Self {
            track_id: item.id,
            artist: artist.to_string(),
            track: track.to_string(),
            album: item
                .album
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            duration_ms: item.duration_ms.filter(|value| *value > 0),
            started_at_unix_secs: now_unix_secs,
            played_ms: 0,
            last_tick_at_ms: Some(now_ms),
            scrobbled: false,
            now_playing_sent: false,
        })
    }

    pub fn refresh_duration(&mut self, duration_ms: u64) {
        if duration_ms > 0 {
            self.duration_ms = Some(duration_ms);
        }
    }

    pub fn should_scrobble(&self) -> bool {
        scrobble_threshold_ms(self.duration_ms).is_some_and(|threshold| self.played_ms >= threshold)
    }

    pub fn to_scrobble_record(&self, next_retry_at_ms: u64) -> LastFmScrobbleRecord {
        LastFmScrobbleRecord {
            track_id: self.track_id,
            artist: self.artist.clone(),
            track: self.track.clone(),
            album: self.album.clone(),
            duration_ms: self.duration_ms,
            started_at_unix_secs: self.started_at_unix_secs,
            retry_count: 0,
            next_retry_at_ms,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LastFmState {
    pub configured: bool,
    pub session: Option<LastFmSession>,
    pub current_playback: Option<LastFmCurrentPlayback>,
    pub scrobble_queue: Vec<LastFmScrobbleRecord>,
    pub auth_inflight: bool,
    pub queue_flush_inflight: bool,
    pub error: Option<String>,
}

impl Default for LastFmState {
    fn default() -> Self {
        Self {
            configured: LastFmBuildConfig::from_env().is_configured(),
            session: None,
            current_playback: None,
            scrobble_queue: Vec::new(),
            auth_inflight: false,
            queue_flush_inflight: false,
            error: None,
        }
    }
}

impl LastFmState {
    pub fn status_label(&self) -> String {
        if !self.configured {
            return "未配置".to_string();
        }
        if self.auth_inflight {
            return "连接中".to_string();
        }
        match self
            .session
            .as_ref()
            .and_then(|session| session.user_name.as_deref())
        {
            Some(user_name) if !user_name.trim().is_empty() => format!("已连接 @{user_name}"),
            _ if self.session.is_some() => "已连接（验证中）".to_string(),
            _ => "未连接".to_string(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.session.is_some()
    }

    pub fn queue_summary(&self) -> String {
        format!("待补发 Scrobble: {}", self.scrobble_queue.len())
    }

    pub fn clear_session(&mut self) {
        self.session = None;
        self.current_playback = None;
        self.scrobble_queue.clear();
        self.auth_inflight = false;
        self.queue_flush_inflight = false;
        self.error = None;
    }

    pub fn next_due_scrobble(&self, now_ms: u64) -> Option<LastFmScrobbleRecord> {
        self.scrobble_queue
            .iter()
            .find(|record| record.next_retry_at_ms <= now_ms)
            .cloned()
    }

    pub fn enqueue_scrobble(&mut self, record: LastFmScrobbleRecord) -> bool {
        if self
            .scrobble_queue
            .iter()
            .any(|existing| existing.matches_identity(&record))
        {
            return false;
        }
        self.scrobble_queue.push(record);
        true
    }

    pub fn remove_scrobble(&mut self, record: &LastFmScrobbleRecord) -> bool {
        let before = self.scrobble_queue.len();
        self.scrobble_queue
            .retain(|existing| !existing.matches_identity(record));
        before != self.scrobble_queue.len()
    }

    pub fn update_scrobble_retry(
        &mut self,
        record: &LastFmScrobbleRecord,
        retry_count: u32,
        next_retry_at_ms: u64,
    ) -> bool {
        let Some(existing) = self
            .scrobble_queue
            .iter_mut()
            .find(|existing| existing.matches_identity(record))
        else {
            return false;
        };
        existing.retry_count = retry_count;
        existing.next_retry_at_ms = next_retry_at_ms;
        true
    }
}

pub fn scrobble_threshold_ms(duration_ms: Option<u64>) -> Option<u64> {
    let duration_ms = duration_ms?;
    if duration_ms < 30_000 {
        return None;
    }
    Some((duration_ms / 2).min(240_000))
}

#[cfg(test)]
mod tests {
    use super::{
        LastFmBuildConfig, LastFmCurrentPlayback, LastFmScrobbleRecord, scrobble_threshold_ms,
    };
    use crate::domain::player::QueueItem;

    #[test]
    fn scrobble_threshold_matches_lastfm_rule() {
        assert_eq!(scrobble_threshold_ms(Some(29_000)), None);
        assert_eq!(scrobble_threshold_ms(Some(30_000)), Some(15_000));
        assert_eq!(scrobble_threshold_ms(Some(600_000)), Some(240_000));
    }

    #[test]
    fn build_config_defaults_to_optional_envs() {
        let _ = LastFmBuildConfig::from_env();
    }

    #[test]
    fn playback_candidate_requires_artist_and_track() {
        let invalid = QueueItem {
            id: 1,
            name: String::new(),
            alias: None,
            artist: "Artist".to_string(),
            album: None,
            duration_ms: Some(180_000),
            cover_url: None,
            source_url: None,
        };
        assert!(LastFmCurrentPlayback::from_queue_item(&invalid, 1, 1).is_none());
    }

    #[test]
    fn scrobble_record_identity_uses_track_and_timestamp() {
        let left = LastFmScrobbleRecord {
            track_id: 1,
            artist: "Artist".to_string(),
            track: "Song".to_string(),
            album: None,
            duration_ms: Some(180_000),
            started_at_unix_secs: 42,
            retry_count: 0,
            next_retry_at_ms: 0,
        };
        let right = left.clone();
        assert!(left.matches_identity(&right));
    }
}
