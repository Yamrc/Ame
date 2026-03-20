mod track;

use std::rc::Rc;
use std::sync::Arc;

use nekowg::{
    AnyElement, App, FontWeight, ListSizingBehavior, MouseButton, ScrollHandle, div, prelude::*,
    px, rgb,
};

use crate::component::{button, theme, virtual_list};
use crate::page::next::models::NextPageSnapshot;

use self::track::queue_track_row;

pub(crate) type QueueItemActionHandler = Rc<dyn Fn(i64, &mut App)>;
pub(crate) type QueueActionHandler = Rc<dyn Fn(&mut App)>;

pub(crate) fn render_next_page(
    snapshot: NextPageSnapshot,
    page_scroll_handle: &ScrollHandle,
    on_play_item: QueueItemActionHandler,
    on_remove_item: QueueItemActionHandler,
    on_clear_queue: QueueActionHandler,
) -> AnyElement {
    let queue_list = if snapshot.upcoming.is_empty() {
        None
    } else {
        let upcoming = Arc::new(snapshot.upcoming);
        let heights = Arc::new(vec![px(60.); upcoming.len()]);
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
                        nekowg::div().pb(px(4.)).child(queue_track_row(
                            format!("next-queue:row:{index}:track:{}", item.id),
                            item,
                            false,
                            Some(Rc::new(move |cx| on_play_item(play_id, cx))),
                            Some(Rc::new(move |cx| on_remove_item(remove_id, cx))),
                        ))
                    })
                    .collect::<Vec<_>>()
            },
        )
        .with_external_viewport_scroll(page_scroll_handle)
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
        queue_track_row(
            format!("next-now-playing:track:{}", track.id),
            track,
            true,
            None,
            None,
        )
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
