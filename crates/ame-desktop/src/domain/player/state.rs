use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackMode {
    Sequence,
    SingleRepeat,
    Shuffle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub id: i64,
    pub name: String,
    pub alias: Option<String>,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: Option<u64>,
    pub cover_url: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlayerEntity {
    pub mode: PlaybackMode,
    pub queue: Vec<QueueItem>,
    pub current_index: Option<usize>,
    pub is_playing: bool,
    pub volume: f32,
    pub position_ms: u64,
    pub duration_ms: u64,
    queue_index_by_id: HashMap<i64, usize>,
    shuffle_seed: u64,
}

impl Default for PlayerEntity {
    fn default() -> Self {
        Self {
            mode: PlaybackMode::Sequence,
            queue: Vec::new(),
            current_index: None,
            is_playing: false,
            volume: 0.7,
            position_ms: 0,
            duration_ms: 180_000,
            queue_index_by_id: HashMap::new(),
            shuffle_seed: 0x9E37_79B9_7F4A_7C15,
        }
    }
}

impl PlayerEntity {
    pub fn apply_audio_snapshot(&mut self, snapshot: &ame_audio::AudioSnapshot) {
        self.is_playing = snapshot.is_playing;
        self.volume = snapshot.volume.clamp(0.0, 1.0);
        self.position_ms = snapshot.position_ms;
        self.duration_ms = snapshot.duration_ms;
    }

    pub fn enqueue(&mut self, item: QueueItem) {
        let next_index = self.queue.len();
        self.queue_index_by_id.insert(item.id, next_index);
        self.queue.push(item);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    pub fn index_of_id(&self, id: i64) -> Option<usize> {
        let index = *self.queue_index_by_id.get(&id)?;
        self.queue
            .get(index)
            .and_then(|item| (item.id == id).then_some(index))
    }

    pub fn remove_at(&mut self, index: usize) {
        if index >= self.queue.len() {
            return;
        }

        self.queue.remove(index);
        match self.current_index {
            Some(_) if self.queue.is_empty() => self.current_index = None,
            Some(current) if current > index => self.current_index = Some(current - 1),
            Some(current) if current == index && current >= self.queue.len() => {
                self.current_index = self.queue.len().checked_sub(1);
            }
            _ => {}
        }
        self.rebuild_queue_index();
    }

    pub fn set_queue(&mut self, queue: Vec<QueueItem>) {
        self.queue = queue;
        self.current_index = self
            .current_index
            .and_then(|index| (index < self.queue.len()).then_some(index))
            .or_else(|| (!self.queue.is_empty()).then_some(0));
        self.rebuild_queue_index();
    }

    pub fn rebuild_queue_index(&mut self) {
        self.queue_index_by_id.clear();
        self.queue_index_by_id.reserve(self.queue.len());
        for (index, item) in self.queue.iter().enumerate() {
            self.queue_index_by_id.entry(item.id).or_insert(index);
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.current_index = None;
        self.queue_index_by_id.clear();
    }

    pub fn cycle_mode(&mut self) {
        self.mode = match self.mode {
            PlaybackMode::Sequence => PlaybackMode::SingleRepeat,
            PlaybackMode::SingleRepeat => PlaybackMode::Shuffle,
            PlaybackMode::Shuffle => PlaybackMode::Sequence,
        };
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0., 1.);
    }

    pub fn current_item(&self) -> Option<&QueueItem> {
        self.current_index.and_then(|index| self.queue.get(index))
    }

    pub fn progress_ratio(&self) -> f32 {
        if self.duration_ms == 0 {
            return 0.;
        }
        (self.position_ms as f32 / self.duration_ms as f32).clamp(0., 1.)
    }

    pub fn next_index(&mut self) -> Option<usize> {
        let len = self.queue.len();
        if len == 0 {
            return None;
        }

        let current = self.current_index.unwrap_or(0);
        let next = match self.mode {
            PlaybackMode::Sequence => {
                if current + 1 >= len {
                    0
                } else {
                    current + 1
                }
            }
            PlaybackMode::SingleRepeat => current.min(len - 1),
            PlaybackMode::Shuffle => self.pseudo_random_index(len, current),
        };

        self.current_index = Some(next);
        Some(next)
    }

    pub fn prev_index(&mut self) -> Option<usize> {
        let len = self.queue.len();
        if len == 0 {
            return None;
        }

        let current = self.current_index.unwrap_or(0);
        let prev = match self.mode {
            PlaybackMode::SingleRepeat => current.min(len - 1),
            PlaybackMode::Sequence | PlaybackMode::Shuffle => {
                if current == 0 {
                    len - 1
                } else {
                    current - 1
                }
            }
        };
        self.current_index = Some(prev);
        Some(prev)
    }

    fn pseudo_random_index(&mut self, len: usize, current: usize) -> usize {
        if len == 1 {
            return 0;
        }

        self.shuffle_seed = self
            .shuffle_seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let mut idx = (self.shuffle_seed as usize) % len;
        if idx == current {
            idx = (idx + 1) % len;
        }
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaybackMode, PlayerEntity, QueueItem};

    fn build_player() -> PlayerEntity {
        let mut p = PlayerEntity::default();
        p.enqueue(QueueItem {
            id: 1,
            name: "A".into(),
            alias: None,
            artist: "Artist A".into(),
            album: None,
            duration_ms: None,
            cover_url: None,
            source_url: None,
        });
        p.enqueue(QueueItem {
            id: 2,
            name: "B".into(),
            alias: None,
            artist: "Artist B".into(),
            album: None,
            duration_ms: None,
            cover_url: None,
            source_url: None,
        });
        p.enqueue(QueueItem {
            id: 3,
            name: "C".into(),
            alias: None,
            artist: "Artist C".into(),
            album: None,
            duration_ms: None,
            cover_url: None,
            source_url: None,
        });
        p
    }

    #[test]
    fn sequence_next_wraps() {
        let mut p = build_player();
        p.current_index = Some(2);
        assert_eq!(p.next_index(), Some(0));
    }

    #[test]
    fn single_repeat_sticks() {
        let mut p = build_player();
        p.current_index = Some(1);
        p.mode = PlaybackMode::SingleRepeat;
        assert_eq!(p.next_index(), Some(1));
        assert_eq!(p.prev_index(), Some(1));
    }

    #[test]
    fn shuffle_moves_when_possible() {
        let mut p = build_player();
        p.current_index = Some(1);
        p.mode = PlaybackMode::Shuffle;
        let next = p.next_index().expect("next");
        assert_ne!(next, 1);
    }

    #[test]
    fn cycle_mode_rotates() {
        let mut p = PlayerEntity::default();
        assert_eq!(p.mode, PlaybackMode::Sequence);
        p.cycle_mode();
        assert_eq!(p.mode, PlaybackMode::SingleRepeat);
        p.cycle_mode();
        assert_eq!(p.mode, PlaybackMode::Shuffle);
        p.cycle_mode();
        assert_eq!(p.mode, PlaybackMode::Sequence);
    }
}
