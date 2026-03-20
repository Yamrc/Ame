use nekowg::Context;

use crate::domain::settings::{CloseBehavior, HomeArtistLanguage};
use crate::domain::{settings, shell};

use super::SettingsPageView;

impl SettingsPageView {
    pub(super) fn set_close_behavior(&mut self, value: CloseBehavior, cx: &mut Context<Self>) {
        shell::set_close_behavior(&self.runtime, value, cx);
    }

    pub(super) fn set_home_artist_language(
        &mut self,
        value: HomeArtistLanguage,
        cx: &mut Context<Self>,
    ) {
        settings::set_home_artist_language(&self.runtime, value, cx);
    }
}
