use nekowg::SharedString;

#[derive(Debug, Clone)]
pub struct SettingsViewModel {
    pub close_behavior_label: SharedString,
    pub home_artist_language_label: SharedString,
}
