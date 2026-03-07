use gpui::{AnyElement, App, FontWeight, MouseButton, SharedString, div, prelude::*, px, rgb};

use crate::component::{button, theme};

#[derive(Debug, Clone)]
pub struct SettingsViewModel {
    pub close_behavior_label: SharedString,
}

pub fn render(
    model: SettingsViewModel,
    on_set_hide_to_tray: impl Fn(&mut App) + 'static,
    on_set_ask: impl Fn(&mut App) + 'static,
    on_set_exit: impl Fn(&mut App) + 'static,
) -> AnyElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(32.))
        .gap_6()
        .child(
            div()
                .text_size(px(42.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_TEXT_DARK))
                .child("设置"),
        )
        .child(
            div()
                .w_full()
                .rounded_lg()
                .bg(rgb(theme::COLOR_CARD_DARK))
                .px_4()
                .py_3()
                .flex()
                .items_center()
                .justify_between()
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child(format!("关闭行为: {}", model.close_behavior_label))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(button::pill_base("隐藏到托盘").on_mouse_down(
                            MouseButton::Left,
                            move |_, _, cx| {
                                on_set_hide_to_tray(cx);
                            },
                        ))
                        .child(button::pill_base("每次询问").on_mouse_down(
                            MouseButton::Left,
                            move |_, _, cx| {
                                on_set_ask(cx);
                            },
                        ))
                        .child(button::pill_base("直接退出").on_mouse_down(
                            MouseButton::Left,
                            move |_, _, cx| {
                                on_set_exit(cx);
                            },
                        )),
                ),
        )
        .into_any_element()
}
