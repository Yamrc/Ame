mod actions;

use std::rc::Rc;

use nekowg::{Context, Render, ScrollHandle, Subscription, Window, prelude::*};

use crate::app::page::PageLifecycle;
use crate::app::runtime::AppRuntime;
use crate::page::next::models::NextPageSnapshot;
use crate::page::next::sections::{QueueActionHandler, QueueItemActionHandler, render_next_page};

pub struct NextPageView {
    runtime: AppRuntime,
    page_scroll_handle: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl NextPageView {
    pub fn new(
        runtime: AppRuntime,
        page_scroll_handle: ScrollHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&runtime.player, |_, _, cx| {
            cx.notify();
        }));
        Self {
            runtime,
            page_scroll_handle,
            _subscriptions: subscriptions,
        }
    }
}

impl Render for NextPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let player_snapshot = self.runtime.player.read(cx).clone();
        let snapshot =
            NextPageSnapshot::from_queue(&player_snapshot.queue, player_snapshot.current_index);
        let page = cx.entity();

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
        let on_clear_queue: QueueActionHandler = {
            let page = page.clone();
            Rc::new(move |cx| {
                page.update(cx, |this, cx| this.clear_queue(cx));
            })
        };

        render_next_page(
            snapshot,
            &self.page_scroll_handle,
            on_play_item,
            on_remove_item,
            on_clear_queue,
        )
    }
}

impl PageLifecycle for NextPageView {}
