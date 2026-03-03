#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackMode {
    Sequence,
    SingleRepeat,
    Shuffle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub id: i64,
    pub name: String,
    pub source_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlayerEntity {
    pub mode: PlaybackMode,
    pub queue: Vec<QueueItem>,
    pub current_index: Option<usize>,
    shuffle_seed: u64,
}

impl Default for PlayerEntity {
    fn default() -> Self {
        Self {
            mode: PlaybackMode::Sequence,
            queue: Vec::new(),
            current_index: None,
            shuffle_seed: 0x9E37_79B9_7F4A_7C15,
        }
    }
}

impl PlayerEntity {
    pub fn enqueue(&mut self, item: QueueItem) {
        self.queue.push(item);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.current_index = None;
    }

    pub fn set_mode(&mut self, mode: PlaybackMode) {
        self.mode = mode;
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
                if current == 0 { len - 1 } else { current - 1 }
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
            source_url: None,
        });
        p.enqueue(QueueItem {
            id: 2,
            name: "B".into(),
            source_url: None,
        });
        p.enqueue(QueueItem {
            id: 3,
            name: "C".into(),
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
        p.set_mode(PlaybackMode::SingleRepeat);
        assert_eq!(p.next_index(), Some(1));
        assert_eq!(p.prev_index(), Some(1));
    }

    #[test]
    fn shuffle_moves_when_possible() {
        let mut p = build_player();
        p.current_index = Some(1);
        p.set_mode(PlaybackMode::Shuffle);
        let next = p.next_index().expect("next");
        assert_ne!(next, 1);
    }
}
