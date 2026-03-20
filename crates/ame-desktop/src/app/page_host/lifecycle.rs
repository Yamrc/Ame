use std::time::{Duration, Instant};

use nekowg::{Context, Pixels, px};
use tracing::error;

use crate::app::page::PageRetentionPolicy;
use crate::app::router;

use super::key::PageKey;
use super::slot::{FrozenEntry, FrozenPage, FrozenSnapshot, PageInstance, create_page};
use super::{FROZEN_TTL, PageHostView};

impl PageHostView {
    pub(crate) fn sync_active_scroll(&mut self, current_scroll: Pixels) {
        if let Some(active) = self.active.as_mut() {
            active.scroll_offset = current_scroll;
        }
    }

    pub(crate) fn take_pending_scroll_restore(&mut self) -> Option<Pixels> {
        self.pending_scroll_restore.take()
    }

    pub(super) fn spawn_prune_task(&self, cx: &mut Context<Self>) {
        let host_entity = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let updated = host_entity.update(cx, |this, cx| {
                    if this.prune_expired(Instant::now(), cx) {
                        cx.notify();
                    }
                });
                if let Err(err) = updated {
                    error!("page host tick failed: {err}");
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn handle_route_change(&mut self, cx: &mut Context<Self>) {
        let route = router::current_route(cx);
        let key = PageKey::from_route(&route);
        let now = Instant::now();

        self.prune_expired(now, cx);

        if let Some(active) = self.active.as_ref()
            && active.key == key
        {
            return;
        }

        if let Some(active) = self.active.take() {
            let snapshot = active.slot.capture_snapshot(cx);
            let retention = active.slot.snapshot_policy(cx);
            active.slot.release_view_resources(cx);

            match retention {
                PageRetentionPolicy::KeepAlive => {
                    self.frozen.insert(
                        active.key,
                        FrozenEntry::KeepAlive(FrozenPage {
                            slot: active.slot,
                            destroy_at: now + FROZEN_TTL,
                            scroll_offset: active.scroll_offset,
                        }),
                    );
                }
                PageRetentionPolicy::SnapshotOnly => {
                    active.slot.on_destroy(cx);
                    self.frozen.insert(
                        active.key,
                        FrozenEntry::Snapshot(Box::new(FrozenSnapshot {
                            snapshot,
                            destroy_at: now + FROZEN_TTL,
                            scroll_offset: active.scroll_offset,
                        })),
                    );
                }
                PageRetentionPolicy::Discard => {
                    active.slot.on_destroy(cx);
                }
            }
        }

        if let Some(frozen) = self.frozen.remove(&key) {
            match frozen {
                FrozenEntry::KeepAlive(frozen) => {
                    let scroll_offset = frozen.scroll_offset;
                    frozen.slot.on_activate(cx);
                    self.active = Some(PageInstance {
                        key,
                        slot: frozen.slot,
                        scroll_offset,
                    });
                    self.pending_scroll_restore = Some(scroll_offset);
                    cx.notify();
                    return;
                }
                FrozenEntry::Snapshot(frozen) => {
                    let frozen = *frozen;
                    let scroll_offset = frozen.scroll_offset;
                    let slot =
                        create_page(&self.runtime, &self.page_scroll_handle, &key, &route, cx);
                    if let Some(snapshot) = frozen.snapshot {
                        slot.restore_snapshot(snapshot, cx);
                    }
                    slot.on_activate(cx);
                    self.active = Some(PageInstance {
                        key,
                        slot,
                        scroll_offset,
                    });
                    self.pending_scroll_restore = Some(scroll_offset);
                    cx.notify();
                    return;
                }
            }
        }

        let slot = create_page(&self.runtime, &self.page_scroll_handle, &key, &route, cx);
        slot.on_activate(cx);
        self.active = Some(PageInstance {
            key,
            slot,
            scroll_offset: px(0.),
        });
        self.pending_scroll_restore = Some(px(0.));
        cx.notify();
    }

    pub(super) fn prune_expired(&mut self, now: Instant, cx: &mut Context<Self>) -> bool {
        let before = self.frozen.len();
        let expired = self
            .frozen
            .iter()
            .filter(|(_, page)| match page {
                FrozenEntry::KeepAlive(page) => page.destroy_at <= now,
                FrozenEntry::Snapshot(page) => page.destroy_at <= now,
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in expired {
            if let Some(page) = self.frozen.remove(&key) {
                match page {
                    FrozenEntry::KeepAlive(page) => page.slot.on_destroy(cx),
                    FrozenEntry::Snapshot(_) => {}
                }
            }
        }
        before != self.frozen.len()
    }
}
