use std::sync::Arc;

use nekowg::{
    AnyElement, App, Div, FontWeight, MouseButton, SharedString, Window, WindowControlArea, div,
    prelude::*, px, rgb, rgba,
};

use crate::component::theme;
use crate::component::{icon, icon::IconName};

#[derive(Debug, Clone)]
pub struct TitleBarModel {
    pub title: SharedString,
    pub is_maximized: bool,
}

#[derive(Clone)]
pub struct TitleBarActions {
    pub on_min: WindowAction,
    pub on_toggle_max_restore: WindowAction,
    pub on_close: WindowAction,
}

type WindowAction = Arc<dyn Fn(&mut Window, &mut App)>;

pub fn title_bar(title: impl Into<SharedString>, controls: AnyElement) -> Div {
    div()
        .flex_none()
        .w_full()
        .h(px(32.))
        .flex()
        .items_center()
        .justify_between()
        .bg(rgba(theme::with_alpha(
            theme::COLOR_NAVBAR_BG_DARK,
            theme::ALPHA_NAVBAR_BG,
        )))
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .window_control_area(WindowControlArea::Drag)
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .px_3()
                        .text_color(rgba(theme::with_alpha(theme::COLOR_TEXT_DARK, 0xFF)))
                        .child(title.into()),
                ),
        )
        .child(div().flex().items_center().gap_1().child(controls))
}

pub fn window_control_button(area: WindowControlArea, icon_name: IconName, is_close: bool) -> Div {
    let icon_size = match icon_name {
        IconName::WindowRestore => 15.,
        _ => 14.,
    };

    let button = div()
        .w(px(46.))
        .h(px(32.))
        .window_control_area(area)
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .child(icon::render(icon_name, icon_size, theme::COLOR_TEXT_DARK));

    if is_close {
        button.hover(|this| this.bg(rgb(0xC42C1B)).text_color(rgb(0xFFFFFF)))
    } else {
        button.hover(|this| this.bg(rgb(theme::COLOR_SECONDARY_BG_DARK)))
    }
}

pub fn window_controls(
    min_button: AnyElement,
    max_button: AnyElement,
    close_button: AnyElement,
) -> Div {
    div()
        .flex()
        .items_center()
        .child(min_button)
        .child(max_button)
        .child(close_button)
}

pub fn render(model: &TitleBarModel, actions: &TitleBarActions) -> AnyElement {
    let max_or_restore = if model.is_maximized {
        IconName::WindowRestore
    } else {
        IconName::WindowMaximize
    };

    let on_min = actions.on_min.clone();
    let on_toggle_max_restore = actions.on_toggle_max_restore.clone();
    let on_close = actions.on_close.clone();

    let min_button = window_control_button(WindowControlArea::Min, IconName::WindowMinimize, false)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| on_min(window, cx))
        .into_any_element();

    let max_button = window_control_button(WindowControlArea::Max, max_or_restore, false)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            on_toggle_max_restore(window, cx)
        })
        .into_any_element();

    let close_button = window_control_button(WindowControlArea::Close, IconName::WindowClose, true)
        .on_mouse_down(MouseButton::Left, move |_, window, cx| on_close(window, cx))
        .into_any_element();

    title_bar(
        model.title.clone(),
        window_controls(min_button, max_button, close_button).into_any_element(),
    )
    .into_any_element()
}
