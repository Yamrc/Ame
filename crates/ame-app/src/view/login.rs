#[derive(Debug, Clone, Default)]
pub struct LoginViewModel {
    pub cookie_input: String,
    pub error: Option<String>,
}
