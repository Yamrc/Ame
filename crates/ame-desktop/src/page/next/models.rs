use crate::domain::player::QueueItem;

#[derive(Debug, Clone)]
pub struct NextPageSnapshot {
    pub current_track: Option<QueueItem>,
    pub upcoming: Vec<QueueItem>,
}

impl NextPageSnapshot {
    pub fn from_queue(queue: &[QueueItem], current_index: Option<usize>) -> Self {
        let current_track = current_index.and_then(|index| queue.get(index).cloned());
        let upcoming = queue
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) > current_index)
            .map(|(_, item)| item.clone())
            .collect();
        Self {
            current_track,
            upcoming,
        }
    }
}
