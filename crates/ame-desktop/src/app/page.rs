use nekowg::Context;

use crate::page::library::LibraryPageFrozenState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageRetentionPolicy {
    KeepAlive,
    SnapshotOnly,
    Discard,
}

#[derive(Debug, Clone)]
pub enum PageSnapshot {
    Library(LibraryPageFrozenState),
    #[doc(hidden)]
    __Reserved,
}

pub trait PageLifecycle: Sized {
    fn on_activate(&mut self, _cx: &mut Context<Self>) {}

    fn snapshot_policy(&self) -> PageRetentionPolicy {
        PageRetentionPolicy::SnapshotOnly
    }

    fn capture_snapshot(&mut self, _cx: &mut Context<Self>) -> Option<PageSnapshot> {
        None
    }

    fn restore_snapshot(
        &mut self,
        _snapshot: PageSnapshot,
        _cx: &mut Context<Self>,
    ) -> Result<(), String> {
        Err("Page does not support snapshot restoration".to_string())
    }

    fn release_view_resources(&mut self, _cx: &mut Context<Self>) {}

    fn on_destroy(&mut self, _cx: &mut Context<Self>) {}
}
