#[derive(Debug, Clone, Default)]
pub struct ControlBarState {
    pub is_playing: bool,
    pub can_next: bool,
    pub can_prev: bool,
}
