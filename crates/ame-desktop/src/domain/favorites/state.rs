use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::service::FavoritesSnapshot;

#[derive(Debug, Clone)]
pub struct FavoritesState {
    pub user_id: Option<i64>,
    pub liked_playlist_id: Option<i64>,
    pub liked_track_ids: Arc<HashSet<i64>>,
    pub pending_updates: Arc<HashMap<i64, bool>>,
    pub loading: bool,
    pub error: Option<String>,
    pub fetched_at_ms: Option<u64>,
    pub change_revision: u64,
}

impl Default for FavoritesState {
    fn default() -> Self {
        Self {
            user_id: None,
            liked_playlist_id: None,
            liked_track_ids: Arc::new(HashSet::new()),
            pending_updates: Arc::new(HashMap::new()),
            loading: false,
            error: None,
            fetched_at_ms: None,
            change_revision: 0,
        }
    }
}

impl FavoritesState {
    pub fn clear(&mut self) {
        self.user_id = None;
        self.liked_playlist_id = None;
        self.liked_track_ids = Arc::new(HashSet::new());
        self.pending_updates = Arc::new(HashMap::new());
        self.loading = false;
        self.error = None;
        self.fetched_at_ms = None;
    }

    pub fn is_ready_for(&self, user_id: Option<i64>) -> bool {
        user_id.is_some() && self.user_id == user_id && self.fetched_at_ms.is_some()
    }

    pub fn is_liked(&self, track_id: i64) -> bool {
        self.pending_updates
            .get(&track_id)
            .copied()
            .unwrap_or_else(|| self.liked_track_ids.contains(&track_id))
    }

    pub fn is_pending(&self, track_id: i64) -> bool {
        self.pending_updates.contains_key(&track_id)
    }

    pub fn begin_loading(&mut self, user_id: i64) {
        if self.user_id != Some(user_id) {
            self.clear();
            self.user_id = Some(user_id);
        }
        self.loading = true;
        self.error = None;
    }

    pub fn apply_snapshot(&mut self, snapshot: FavoritesSnapshot, fetched_at_ms: Option<u64>) {
        self.user_id = Some(snapshot.user_id);
        self.liked_playlist_id = snapshot.liked_playlist_id;
        self.liked_track_ids = Arc::new(snapshot.track_ids.into_iter().collect());
        self.loading = false;
        self.error = None;
        self.fetched_at_ms = Some(fetched_at_ms.unwrap_or(snapshot.fetched_at_ms));
    }

    pub fn fail_preserving_cached(&mut self, error: impl Into<String>) {
        self.loading = false;
        self.error = Some(error.into());
        if self.fetched_at_ms.is_none() {
            self.liked_playlist_id = None;
            self.liked_track_ids = Arc::new(HashSet::new());
        }
    }

    pub fn set_pending(&mut self, track_id: i64, liked: bool) {
        let mut pending = (*self.pending_updates).clone();
        pending.insert(track_id, liked);
        self.pending_updates = Arc::new(pending);
        self.error = None;
    }

    pub fn clear_pending(&mut self, track_id: i64) {
        let mut pending = (*self.pending_updates).clone();
        pending.remove(&track_id);
        self.pending_updates = Arc::new(pending);
    }

    pub fn apply_toggle_success(&mut self, track_id: i64, liked: bool, fetched_at_ms: u64) {
        self.clear_pending(track_id);
        let mut liked_track_ids = (*self.liked_track_ids).clone();
        if liked {
            liked_track_ids.insert(track_id);
        } else {
            liked_track_ids.remove(&track_id);
        }
        self.liked_track_ids = Arc::new(liked_track_ids);
        self.loading = false;
        self.error = None;
        self.fetched_at_ms = Some(fetched_at_ms);
        self.change_revision = self.change_revision.saturating_add(1);
    }

    pub fn fail_toggle(&mut self, track_id: i64, error: impl Into<String>) {
        self.clear_pending(track_id);
        self.loading = false;
        self.error = Some(error.into());
    }

    pub fn snapshot(&self) -> Option<FavoritesSnapshot> {
        let user_id = self.user_id?;
        let fetched_at_ms = self.fetched_at_ms?;
        let mut track_ids = self.liked_track_ids.iter().copied().collect::<Vec<_>>();
        track_ids.sort_unstable();
        Some(FavoritesSnapshot {
            user_id,
            liked_playlist_id: self.liked_playlist_id,
            track_ids,
            fetched_at_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::FavoritesState;

    #[test]
    fn pending_state_overrides_cached_membership() {
        let mut state = FavoritesState {
            user_id: Some(1),
            fetched_at_ms: Some(1),
            ..FavoritesState::default()
        };
        state.set_pending(42, true);

        assert!(state.is_liked(42));
        assert!(state.is_pending(42));
    }

    #[test]
    fn successful_toggle_updates_change_revision() {
        let mut state = FavoritesState {
            user_id: Some(1),
            fetched_at_ms: Some(1),
            ..FavoritesState::default()
        };
        state.set_pending(42, true);
        state.apply_toggle_success(42, true, 2);

        assert!(state.is_liked(42));
        assert_eq!(state.change_revision, 1);
        assert!(!state.is_pending(42));
    }
}
