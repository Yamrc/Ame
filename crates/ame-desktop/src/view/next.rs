use gpui::{AnyElement, App, FontWeight, MouseButton, div, prelude::*, px, rgb};

use crate::component::{button, theme};
use crate::entity::player::QueueItem;

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

pub fn render(
    current_track: Option<QueueItem>,
    clear_button: Option<AnyElement>,
    queue_list: Option<AnyElement>,
) -> AnyElement {
    let now_playing = if let Some(track) = current_track {
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
