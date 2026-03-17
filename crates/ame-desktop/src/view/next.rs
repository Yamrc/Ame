use std::sync::Arc;

use nekowg::{
    AnyElement, App, Context, Entity, FontWeight, ListSizingBehavior, MouseButton, Render,
    ScrollHandle, Subscription, Window, div, prelude::*, px, rgb,
};

use crate::component::{button, theme, virtual_list};
use crate::entity::player::QueueItem;
use crate::entity::player_controller::PlayerController;
use crate::entity::runtime::AppRuntime;

type QueueItemActionHandler = Arc<dyn Fn(i64, &mut App)>;
type QueueActionHandler = Arc<dyn Fn(&mut App)>;

#[derive(Debug, Clone)]
pub struct NextPageSnapshot {
    pub current_track: Option<QueueItem>,
    pub upcoming: Vec<QueueItem>,
}

impl NextPageSnapshot {
    pub fn from_queue(queue: &[QueueItem], current_index: Option<usize>) -> Self {
        let current_track = current_index.and_then(|index| queue.get(index).cloned());
        let upcoming = queue
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) > current_index)
            .map(|(_, item)| item.clone())
            .collect();
        Self {
            current_track,
            upcoming,
        }
    }
}

pub fn queue_row(
    item: QueueItem,
    on_play: impl Fn(&mut App) + 'static,
    on_remove: impl Fn(&mut App) + 'static,
) -> AnyElement {
    div()
        .w_full()
        .rounded_lg()
        .bg(rgb(theme::COLOR_CARD_DARK))
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .child(div().font_weight(FontWeight::BOLD).child(item.name))
                .child(
                    div()
                        .text_color(rgb(theme::COLOR_SECONDARY))
                        .child(format!("ID {}", item.id)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    button::pill_base("播放")
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_play(cx)),
                )
                .child(
                    button::pill_base("移除")
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_remove(cx)),
                ),
        )
        .into_any_element()
}

pub struct NextPageView {
    runtime: AppRuntime,
    player_controller: Entity<PlayerController>,
    page_scroll_handle: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl NextPageView {
    pub fn new(
        runtime: AppRuntime,
        player_controller: Entity<PlayerController>,
        page_scroll_handle: ScrollHandle,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            runtime,
            player_controller,
            page_scroll_handle,
            _subscriptions: Vec::new(),
        }
    }
}

impl Render for NextPageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let player_snapshot = self.runtime.player.read(cx).clone();
        let snapshot =
            NextPageSnapshot::from_queue(&player_snapshot.queue, player_snapshot.current_index);

        let on_play_item: QueueItemActionHandler = {
            let player_controller = self.player_controller.clone();
            Arc::new(move |item_id, cx| {
                player_controller.update(cx, |this, cx| this.play_queue_item(item_id, cx));
            })
        };
        let on_remove_item: QueueItemActionHandler = {
            let player_controller = self.player_controller.clone();
            Arc::new(move |item_id, cx| {
                player_controller.update(cx, |this, cx| this.remove_queue_item(item_id, cx));
            })
        };
        let on_clear_queue: QueueActionHandler = {
            let player_controller = self.player_controller.clone();
            Arc::new(move |cx| {
                player_controller.update(cx, |this, cx| this.clear_queue(cx));
            })
        };

        let queue_list = if snapshot.upcoming.is_empty() {
            None
        } else {
            let upcoming = Arc::new(snapshot.upcoming);
            let heights = Arc::new(vec![px(88.); upcoming.len()]);
            let on_play_item = on_play_item.clone();
            let on_remove_item = on_remove_item.clone();
            let list = virtual_list::v_virtual_list(
                ("next-queue", upcoming.len()),
                heights,
                move |visible_range, _, _| {
                    visible_range
                        .map(|index| {
                            let item = upcoming[index].clone();
                            let play_id = item.id;
                            let remove_id = item.id;
                            let on_play_item = on_play_item.clone();
                            let on_remove_item = on_remove_item.clone();
                            nekowg::div().pb(px(8.)).child(queue_row(
                                item,
                                move |cx| on_play_item(play_id, cx),
                                move |cx| on_remove_item(remove_id, cx),
                            ))
                        })
                        .collect::<Vec<_>>()
                },
            )
            .with_external_viewport_scroll(&self.page_scroll_handle)
            .with_sizing_behavior(ListSizingBehavior::Infer)
            .with_overscan(2)
            .w_full();
            Some(list.into_any_element())
        };

        let clear_button = if snapshot.current_track.is_some() || queue_list.is_some() {
            let on_clear_queue = on_clear_queue.clone();
            Some(
                button::pill_base("清空队列")
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| on_clear_queue(cx))
                    .into_any_element(),
            )
        } else {
            None
        };

        let now_playing = if let Some(track) = snapshot.current_track {
            div()
                .w_full()
                .rounded_lg()
                .bg(rgb(theme::COLOR_CARD_DARK))
                .px_4()
                .py_3()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().font_weight(FontWeight::BOLD).child(track.name))
                .child(
                    div()
                        .text_color(rgb(theme::COLOR_SECONDARY))
                        .child(format!("ID {}", track.id)),
                )
                .into_any_element()
        } else {
            div()
                .w_full()
                .rounded_lg()
                .bg(rgb(theme::COLOR_CARD_DARK))
                .px_4()
                .py_3()
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child("暂无正在播放")
                .into_any_element()
        };

        let queue_list = queue_list.unwrap_or_else(|| {
            div()
                .w_full()
                .rounded_lg()
                .bg(rgb(theme::COLOR_CARD_DARK))
                .px_4()
                .py_3()
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child("暂无待播放队列")
                .into_any_element()
        });

        let next_header = div()
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(28.))
                    .font_weight(FontWeight::BOLD)
                    .child("接下来播放"),
            )
            .child(clear_button.unwrap_or_else(|| div().into_any_element()))
            .into_any_element();

        div()
            .w_full()
            .flex()
            .flex_col()
            .pt(px(24.))
            .pb(px(32.))
            .gap_6()
            .child(
                div()
                    .text_size(px(42.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                    .child("播放队列"),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .text_size(px(28.))
                            .font_weight(FontWeight::BOLD)
                            .child("正在播放"),
                    )
                    .child(now_playing),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(next_header)
                    .child(queue_list),
            )
            .into_any_element()
    }
}
