mod actions;

use std::rc::Rc;
use std::sync::Arc;

use nekowg::{Context, Render, ScrollHandle, Subscription, Window, prelude::*, px};

use crate::app::page::{PageLifecycle, PageRetentionPolicy};
use crate::app::runtime::AppRuntime;
use crate::domain::favorites;
use crate::page::next::sections::{
    NextQueueActions, NextQueueFavoriteState, NextQueueRenderCache, QueueActionHandler,
    QueueItemActionHandler, render_next_page,
};

pub struct NextPageView {
    runtime: AppRuntime,
    page_scroll_handle: ScrollHandle,
    heavy_resources: NextQueueRenderCache,
    _subscriptions: Vec<Subscription>,
}

impl NextPageView {
    pub fn new(
        runtime: AppRuntime,
        page_scroll_handle: ScrollHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut view = Self {
            runtime,
            page_scroll_handle,
            heavy_resources: NextQueueRenderCache {
                current_track: None,
                upcoming: Arc::new(Vec::new()),
                heights: Arc::new(Vec::new()),
            },
            _subscriptions: Vec::new(),
        };
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&view.runtime.player, |this, _, cx| {
            this.refresh_heavy_resources(cx);
            cx.notify();
        }));
        subscriptions.push(cx.observe(&view.runtime.favorites, |_, _, cx| {
            cx.notify();
        }));
        view._subscriptions = subscriptions;
        view.refresh_heavy_resources(cx);
        view
    }

    fn refresh_heavy_resources(&mut self, cx: &mut Context<Self>) {
        let player = self.runtime.player.read(cx);
        let upcoming = player
            .queue
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) > player.current_index)
            .map(|(_, item)| item.clone())
            .collect::<Vec<_>>();
        self.heavy_resources = NextQueueRenderCache {
            current_track: player
                .current_index
                .and_then(|index| player.queue.get(index).cloned()),
            heights: Arc::new(vec![px(60.); upcoming.len()]),
            upcoming: Arc::new(upcoming),
        };
    }
}

impl Render for NextPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = cx.entity();
        let session_user_id = self.runtime.session.read(cx).auth_user_id;
        let favorites_state = self.runtime.favorites.read(cx).clone();
        let favorite_ready = favorites_state.is_ready_for(session_user_id);

        let on_play_item: QueueItemActionHandler = {
            let page = page.clone();
            Rc::new(move |item_id, cx| {
                page.update(cx, |this, cx| this.play_item(item_id, cx));
            })
        };
        let on_remove_item: QueueItemActionHandler = {
            let page = page.clone();
            Rc::new(move |item_id, cx| {
                page.update(cx, |this, cx| this.remove_item(item_id, cx));
            })
        };
        let on_toggle_favorite: QueueItemActionHandler = {
            let page = page.clone();
            Rc::new(move |item_id, cx| {
                page.update(cx, |this, cx| {
                    favorites::toggle_track_like(&this.runtime, item_id, cx);
                });
            })
        };
        let on_clear_queue: QueueActionHandler = {
            let page = page.clone();
            Rc::new(move |cx| {
                page.update(cx, |this, cx| this.clear_queue(cx));
            })
        };

        render_next_page(
            &self.heavy_resources,
            &self.page_scroll_handle,
            NextQueueFavoriteState {
                favorites: favorites_state,
                ready: favorite_ready,
            },
            NextQueueActions {
                on_play_item,
                on_toggle_favorite,
                on_remove_item,
                on_clear_queue,
            },
        )
    }
}

impl PageLifecycle for NextPageView {
    fn snapshot_policy(&self) -> PageRetentionPolicy {
        PageRetentionPolicy::SnapshotOnly
    }

    fn release_view_resources(&mut self, _cx: &mut Context<Self>) {
        self.heavy_resources = NextQueueRenderCache {
            current_track: None,
            upcoming: Arc::new(Vec::new()),
            heights: Arc::new(Vec::new()),
        };
    }
}
