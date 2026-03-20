#[derive(Debug, Clone, Default)]
pub struct ShellState {
    pub error: Option<String>,
    pub close_behavior: crate::domain::settings::CloseBehavior,
}
