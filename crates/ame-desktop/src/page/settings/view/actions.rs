use nekowg::Context;

use crate::domain::lastfm;
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

    pub(super) fn refresh_login(&mut self, cx: &mut Context<Self>) {
        crate::domain::session::refresh_login_token(&self.runtime, cx);
    }

    pub(super) fn connect_lastfm(&mut self, cx: &mut Context<Self>) {
        lastfm::connect(&self.runtime, cx);
    }

    pub(super) fn disconnect_lastfm(&mut self, cx: &mut Context<Self>) {
        lastfm::disconnect(&self.runtime, cx);
    }
}
