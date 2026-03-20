mod actions;

use std::rc::Rc;

use nekowg::{Context, Render, Subscription, Window, prelude::*};

use crate::app::page::PageLifecycle;
use crate::app::runtime::AppRuntime;
use crate::page::settings::models::SettingsViewModel;
use crate::page::settings::sections::{
    CloseBehaviorHandler, HomeArtistLanguageHandler, render_settings_page,
};

pub struct SettingsPageView {
    runtime: AppRuntime,
    _subscriptions: Vec<Subscription>,
}

impl SettingsPageView {
    pub fn new(runtime: AppRuntime, _cx: &mut Context<Self>) -> Self {
        Self {
            runtime,
            _subscriptions: Vec::new(),
        }
    }
}

impl Render for SettingsPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = SettingsViewModel {
            close_behavior_label: self.runtime.shell.read(cx).close_behavior.label().into(),
            home_artist_language_label: self
                .runtime
                .app
                .read(cx)
                .home_artist_language
                .label()
                .into(),
        };
        let page = cx.entity();
        let on_set_close_behavior: CloseBehaviorHandler = Rc::new(move |value, cx| {
            page.update(cx, |this, cx| this.set_close_behavior(value, cx));
        });
        let page = cx.entity();
        let on_set_home_artist_language: HomeArtistLanguageHandler = Rc::new(move |value, cx| {
            page.update(cx, |this, cx| this.set_home_artist_language(value, cx));
        });

        render_settings_page(model, on_set_close_behavior, on_set_home_artist_language)
    }
}

impl PageLifecycle for SettingsPageView {}
