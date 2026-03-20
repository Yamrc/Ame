#[derive(Debug, Clone, Default)]
pub struct AppEntity {
    pub search_query: String,
    pub home_artist_language: crate::domain::settings::HomeArtistLanguage,
}

impl AppEntity {
    pub fn set_search_query(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
    }

    pub fn set_home_artist_language(
        &mut self,
        language: crate::domain::settings::HomeArtistLanguage,
    ) {
        self.home_artist_language = language;
    }
}
