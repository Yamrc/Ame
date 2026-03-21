use nekowg::{AnyElement, AppContext, IntoElement};
use tracing::error;

use crate::app::page::{PageLifecycle, PageRetentionPolicy, PageSnapshot};

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

    pub(in crate::app::page_host) fn snapshot_policy<C: AppContext>(
        &self,
        cx: &mut C,
    ) -> PageRetentionPolicy {
        match self {
            Self::Home(view) => view.update(cx, |this, _| this.snapshot_policy()),
            Self::Discover(view) => view.update(cx, |this, _| this.snapshot_policy()),
            Self::Library(view) => view.update(cx, |this, _| this.snapshot_policy()),
            Self::Search(view) => view.update(cx, |this, _| this.snapshot_policy()),
            Self::DailyTracks(view) => view.update(cx, |this, _| this.snapshot_policy()),
            Self::Next(view) => view.update(cx, |this, _| this.snapshot_policy()),
            Self::Playlist(view) => view.update(cx, |this, _| this.snapshot_policy()),
            Self::Settings(view) => view.update(cx, |this, _| this.snapshot_policy()),
            Self::Login(view) => view.update(cx, |this, _| this.snapshot_policy()),
            Self::Unknown(view) => view.update(cx, |this, _| this.snapshot_policy()),
        }
    }

    pub(in crate::app::page_host) fn capture_snapshot<C: AppContext>(
        &self,
        cx: &mut C,
    ) -> Option<PageSnapshot> {
        match self {
            Self::Home(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
            Self::Discover(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
            Self::Library(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
            Self::Search(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
            Self::DailyTracks(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
            Self::Next(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
            Self::Playlist(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
            Self::Settings(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
            Self::Login(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
            Self::Unknown(view) => view.update(cx, |this, cx| this.capture_snapshot(cx)),
        }
    }

    pub(in crate::app::page_host) fn restore_snapshot<C: AppContext>(
        &self,
        snapshot: PageSnapshot,
        cx: &mut C,
    ) {
        let result = match self {
            Self::Home(view) => view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx)),
            Self::Discover(view) => view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx)),
            Self::Library(view) => view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx)),
            Self::Search(view) => view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx)),
            Self::DailyTracks(view) => {
                view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx))
            }
            Self::Next(view) => view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx)),
            Self::Playlist(view) => view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx)),
            Self::Settings(view) => view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx)),
            Self::Login(view) => view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx)),
            Self::Unknown(view) => view.update(cx, |this, cx| this.restore_snapshot(snapshot, cx)),
        };
        if let Err(err) = result {
            error!("Failed to restore page snapshot: {err}");
        }
    }

    pub(in crate::app::page_host) fn release_view_resources<C: AppContext>(&self, cx: &mut C) {
        match self {
            Self::Home(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
            Self::Discover(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
            Self::Library(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
            Self::Search(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
            Self::DailyTracks(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
            Self::Next(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
            Self::Playlist(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
            Self::Settings(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
            Self::Login(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
            Self::Unknown(view) => view.update(cx, |this, cx| this.release_view_resources(cx)),
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
