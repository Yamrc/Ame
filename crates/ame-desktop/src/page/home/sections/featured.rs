use nekowg::{
    AnyElement, App, FontWeight, MouseButton, div, img, linear_color_stop, linear_gradient,
    prelude::*, px, rgb, rgba,
};

use crate::component::{
    button,
    icon::{self, IconName},
    theme,
};
use crate::page::home::models::HomePlaylistCard;
use crate::util::url::image_resize_url;

pub(super) fn daily_featured_card(
    item: HomePlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
    on_play: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let cover = item.cover_url.clone();
    div()
        .w_full()
        .h(px(198.))
        .rounded_xl()
        .overflow_hidden()
        .cursor_pointer()
        .relative()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx))
        .child(match cover {
            Some(url) => img(image_resize_url(&url, "512y512"))
                .id(format!("home-daily-featured-{url}"))
                .w_full()
                .h_full()
                .rounded_xl()
                .into_any_element(),
            None => div()
                .w_full()
                .h_full()
                .rounded_xl()
                .bg(rgb(0x3B3B3B))
                .into_any_element(),
        })
        .child(
            div()
                .absolute()
                .left(px(0.))
                .top(px(0.))
                .right(px(0.))
                .bottom(px(0.))
                .h_full()
                .px(px(24.))
                .py(px(20.))
                .bg(rgba(theme::with_alpha(0x000000, 0x2E)))
                .flex()
                .items_center()
                .child(
                    div()
                        .w(px(148.))
                        .h(px(148.))
                        .text_size(px(64.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(theme::COLOR_TEXT_DARK))
                        .grid()
                        .grid_cols(2)
                        .justify_center()
                        .items_center()
                        .line_height(px(52.))
                        .children(["每", "日", "推", "荐"]),
                ),
        )
        .child(
            div()
                .absolute()
                .right(px(20.))
                .bottom(px(18.))
                .size(px(44.))
                .rounded_full()
                .bg(rgba(theme::with_alpha(0xFFFFFF, 0x38)))
                .flex()
                .justify_center()
                .items_center()
                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    on_play(cx);
                })
                .child(icon::render(IconName::Play, 18.0, theme::COLOR_TEXT_DARK)),
        )
        .into_any_element()
}

pub(super) fn fm_featured_card(
    item: HomePlaylistCard,
    on_open: impl Fn(&mut App) + 'static,
) -> AnyElement {
    let cover = item.cover_url.clone();
    let mut card = div()
        .w_full()
        .h(px(198.))
        .rounded_xl()
        .overflow_hidden()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx));

    if let Some(url) = cover.as_deref() {
        card = card.bg(gradient_from_seed(url));
    } else {
        card = card.bg(rgb(0x8D8D8D));
    }

    card.child(
        div()
            .size_full()
            .px(px(16.))
            .py(px(14.))
            .flex()
            .gap(px(16.))
            .child(match cover {
                Some(url) => img(image_resize_url(&url, "256y256"))
                    .id(format!("home-fm-featured-{url}"))
                    .w(px(169.))
                    .h(px(169.))
                    .flex_shrink_0()
                    .rounded_lg()
                    .overflow_hidden()
                    .into_any_element(),
                None => div()
                    .w(px(169.))
                    .h(px(169.))
                    .flex_shrink_0()
                    .rounded_lg()
                    .bg(rgb(0x6F6F6F))
                    .into_any_element(),
            })
            .child(
                div()
                    .flex_grow()
                    .min_w(px(0.))
                    .h_full()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(
                        div()
                            .pt(px(4.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .w_full()
                                    .text_size(px(28.))
                                    .line_height(px(28.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                                    .truncate()
                                    .child(item.name),
                            )
                            .child(
                                div()
                                    .mt(px(4.))
                                    .text_size(px(15.))
                                    .text_color(rgba(theme::with_alpha(0xFFFFFF, 0xA8)))
                                    .overflow_hidden()
                                    .child(item.subtitle),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_between()
                            .items_end()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.))
                                    .child(icon_button(IconName::ThumbsDown))
                                    .child(icon_button(IconName::Play))
                                    .child(icon_button(IconName::Next)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(16.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .opacity(0.38)
                                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                                    .child(icon::render(IconName::Fm, 16.0, theme::COLOR_TEXT_DARK))
                                    .child("私人FM"),
                            ),
                    ),
            ),
    )
    .into_any_element()
}

fn icon_button(icon_name: IconName) -> AnyElement {
    let style = button::ButtonStyle {
        padding: px(0.),
        margin: px(0.),
        radius: px(8.),
        base_bg: button::transparent_bg(),
        hover_bg: rgba(theme::with_alpha(0xFFFFFF, 0x18)),
        hover_duration_ms: 180,
    };

    button::icon_interactive(
        format!("home-fm-icon-{icon_name:?}"),
        button::icon_base(style).size(px(34.)).child(icon::render(
            icon_name,
            18.0,
            theme::COLOR_TEXT_DARK,
        )),
        style,
    )
    .into_any_element()
}

fn gradient_from_seed(seed: &str) -> nekowg::Background {
    let base = color_from_seed(seed);
    let darker = shift_color(base, -32);
    let lighter = shift_color(base, 28);
    linear_gradient(
        120.0,
        linear_color_stop(rgb(lighter), 0.0),
        linear_color_stop(rgb(darker), 1.0),
    )
}

fn color_from_seed(seed: &str) -> u32 {
    let mut hash = 2166136261u32;
    for byte in seed.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    let r = ((hash >> 16) & 0xFF) as u8;
    let g = ((hash >> 8) & 0xFF) as u8;
    let b = (hash & 0xFF) as u8;
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn shift_color(color: u32, delta: i16) -> u32 {
    let r = ((color >> 16) & 0xFF) as i16 + delta;
    let g = ((color >> 8) & 0xFF) as i16 + delta;
    let b = (color & 0xFF) as i16 + delta;
    let r = r.clamp(0, 255) as u32;
    let g = g.clamp(0, 255) as u32;
    let b = b.clamp(0, 255) as u32;
    (r << 16) | (g << 8) | b
}
