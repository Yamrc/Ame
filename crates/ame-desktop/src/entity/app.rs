use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CloseBehavior {
    #[default]
    Ask,
    HideToTray,
    Exit,
}

impl CloseBehavior {
    pub const fn label(self) -> &'static str {
        match self {
            Self::HideToTray => "隐藏到托盘",
            Self::Ask => "每次询问",
            Self::Exit => "直接退出",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppEntity {
    pub search_query: String,
}

impl AppEntity {
    pub fn set_search_query(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
    }
}
