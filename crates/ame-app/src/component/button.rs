use gpui::{Div, SharedString, div, prelude::*, px, rgb, rgba};
use gpui_animation::{animation::TransitionExt, transition::general::Linear};
use std::time::Duration;

use crate::component::theme;

#[derive(Debug, Clone, Copy)]
pub struct ButtonStyle {
    pub padding: gpui::Pixels,
    pub margin: gpui::Pixels,
    pub radius: gpui::Pixels,
    pub base_bg: gpui::Rgba,
    pub hover_bg: gpui::Rgba,
    pub hover_duration_ms: u64,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            padding: px(6.),
            margin: px(2.),
            radius: px(8.),
            base_bg: transparent_bg(),
            hover_bg: hover_bg(),
            hover_duration_ms: 200,
        }
    }
}

pub fn icon_base(style: ButtonStyle) -> Div {
    div()
        .flex()
        .justify_center()
        .items_center()
        .p(style.padding)
        .m(style.margin)
        .rounded(style.radius)
        .bg(style.base_bg)
        .cursor_pointer()
}

pub fn icon_interactive(
    id: impl Into<SharedString>,
    button: Div,
    style: ButtonStyle,
) -> impl IntoElement {
    let id: SharedString = id.into();
    button
        .id(id.clone())
        .with_transition(id)
        .transition_on_hover(
            Duration::from_millis(style.hover_duration_ms),
            Linear,
            move |hovered, this| {
                if *hovered {
                    this.bg(style.hover_bg)
                } else {
                    this.bg(style.base_bg)
                }
            },
        )
}

pub fn pill_base(label: impl Into<SharedString>) -> Div {
    div()
        .h_10()
        .px_3()
        .rounded_lg()
        .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
        .text_color(rgb(theme::COLOR_TEXT_DARK))
        .cursor_pointer()
        .flex()
        .items_center()
        .child(label.into())
}

pub fn primary_pill(label: impl Into<SharedString>) -> Div {
    div()
        .h_10()
        .px_4()
        .rounded_lg()
        .bg(rgb(theme::COLOR_PRIMARY))
        .text_color(rgb(theme::COLOR_TEXT_DARK))
        .cursor_pointer()
        .flex()
        .items_center()
        .child(label.into())
}

pub fn chip_base(label: impl Into<SharedString>, active: bool) -> Div {
    div()
        .px(px(14.))
        .py(px(6.))
        .rounded(px(10.))
        .bg(if active {
            rgb(theme::COLOR_PRIMARY_BG_DARK)
        } else {
            rgb(theme::COLOR_SECONDARY_BG_DARK)
        })
        .text_color(if active {
            rgb(theme::COLOR_PRIMARY)
        } else {
            rgb(theme::COLOR_SECONDARY)
        })
        .text_size(px(18.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .cursor_pointer()
        .child(label.into())
}

pub fn transparent_bg() -> gpui::Rgba {
    rgba(theme::with_alpha(theme::COLOR_BODY_BG_DARK, 0x00))
}

pub fn hover_bg() -> gpui::Rgba {
    rgba(theme::with_alpha(
        theme::COLOR_SECONDARY_BG_TRANSPARENT_DARK,
        theme::ALPHA_SECONDARY_BG_TRANSPARENT,
    ))
}
