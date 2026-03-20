use nekowg::{AnyElement, AppContext, IntoElement};

use crate::app::page::PageLifecycle;

use super::PageSlot;

impl PageSlot {
    pub(in crate::app::page_host) fn element(&self) -> AnyElement {
        match self {
            Self::Home(view) => view.clone().into_any_element(),
            Self::Discover(view) => view.clone().into_any_element(),
            Self::Library(view) => view.clone().into_any_element(),
            Self::Search(view) => view.clone().into_any_element(),
            Self::DailyTracks(view) => view.clone().into_any_element(),
            Self::Next(view) => view.clone().into_any_element(),
            Self::Playlist(view) => view.clone().into_any_element(),
            Self::Settings(view) => view.clone().into_any_element(),
            Self::Login(view) => view.clone().into_any_element(),
            Self::Unknown(view) => view.clone().into_any_element(),
        }
    }

    pub(in crate::app::page_host) fn on_activate<C: AppContext>(&self, cx: &mut C) {
        match self {
            Self::Home(view) => view.update(cx, |this, cx| this.on_activate(cx)),
            Self::Discover(view) => view.update(cx, |this, cx| this.on_activate(cx)),
            Self::Library(view) => view.update(cx, |this, cx| this.on_activate(cx)),
            Self::Search(view) => view.update(cx, |this, cx| this.on_activate(cx)),
            Self::DailyTracks(view) => view.update(cx, |this, cx| this.on_activate(cx)),
            Self::Next(view) => view.update(cx, |this, cx| this.on_activate(cx)),
            Self::Playlist(view) => view.update(cx, |this, cx| this.on_activate(cx)),
            Self::Settings(view) => view.update(cx, |this, cx| this.on_activate(cx)),
            Self::Login(view) => view.update(cx, |this, cx| this.on_activate(cx)),
            Self::Unknown(view) => view.update(cx, |this, cx| this.on_activate(cx)),
        }
    }

    pub(in crate::app::page_host) fn on_frozen<C: AppContext>(&self, cx: &mut C) {
        match self {
            Self::Home(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
            Self::Discover(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
            Self::Library(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
            Self::Search(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
            Self::DailyTracks(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
            Self::Next(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
            Self::Playlist(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
            Self::Settings(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
            Self::Login(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
            Self::Unknown(view) => view.update(cx, |this, cx| this.on_frozen(cx)),
        }
    }

    pub(in crate::app::page_host) fn on_destroy<C: AppContext>(&self, cx: &mut C) {
        match self {
            Self::Home(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
            Self::Discover(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
            Self::Library(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
            Self::Search(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
            Self::DailyTracks(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
            Self::Next(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
            Self::Playlist(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
            Self::Settings(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
            Self::Login(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
            Self::Unknown(view) => view.update(cx, |this, cx| this.on_destroy(cx)),
        }
    }
}
