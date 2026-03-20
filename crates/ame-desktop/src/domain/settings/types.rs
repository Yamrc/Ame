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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HomeArtistLanguage {
    Chinese,
    Western,
    Korean,
    #[default]
    Japanese,
}

impl HomeArtistLanguage {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Chinese => "华语",
            Self::Western => "欧美",
            Self::Korean => "韩语",
            Self::Japanese => "日语",
        }
    }

    pub const fn toplist_type(self) -> u32 {
        match self {
            Self::Chinese => 1,
            Self::Western => 2,
            Self::Korean => 3,
            Self::Japanese => 4,
        }
    }

    pub const fn variants() -> [Self; 4] {
        [Self::Japanese, Self::Chinese, Self::Western, Self::Korean]
    }
}

#[cfg(test)]
mod tests {
    use super::HomeArtistLanguage;

    #[test]
    fn home_artist_language_type_mapping_matches_ypm_table() {
        assert_eq!(HomeArtistLanguage::Chinese.toplist_type(), 1);
        assert_eq!(HomeArtistLanguage::Western.toplist_type(), 2);
        assert_eq!(HomeArtistLanguage::Korean.toplist_type(), 3);
        assert_eq!(HomeArtistLanguage::Japanese.toplist_type(), 4);
    }

    #[test]
    fn home_artist_language_default_is_japanese() {
        assert_eq!(HomeArtistLanguage::default(), HomeArtistLanguage::Japanese);
    }
}
