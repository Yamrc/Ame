use std::sync::Arc;

use nekowg::{
    AnyElement, App, Div, Entity, FontWeight, MouseButton, SharedString, div, img, prelude::*, px,
    relative, rgb, rgba,
};

use crate::component::theme;
use crate::component::{
    button,
    icon::{self, IconName},
    page_scaffold, slider,
};
use crate::domain::player::PlaybackMode;
use crate::util::url::image_resize_url;

pub type BottomBarAction = Arc<dyn Fn(&mut App)>;

#[derive(Debug, Clone)]
pub struct BottomBarModel {
    pub has_current_song: bool,
    pub current_song: SharedString,
    pub current_artist: SharedString,
    pub current_cover_url: Option<SharedString>,
    pub favorite_liked: bool,
    pub favorite_enabled: bool,
    pub favorite_pending: bool,
    pub is_playing: bool,
    pub mode: PlaybackMode,
    pub volume: f32,
    pub progress_slider: Entity<slider::SliderState>,
    pub volume_slider: Entity<slider::SliderState>,
}

#[derive(Clone)]
pub struct BottomBarActions {
    pub on_prev: BottomBarAction,
    pub on_toggle: BottomBarAction,
    pub on_next: BottomBarAction,
    pub on_toggle_favorite: Option<BottomBarAction>,
    pub on_open_queue: BottomBarAction,
    pub on_cycle_mode: BottomBarAction,
}

pub fn left_section(content: AnyElement) -> Div {
    div()
        .h_full()
        .flex()
        .items_center()
        .justify_start()
        .child(content)
}

pub fn center_section(content: AnyElement) -> Div {
    div()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .child(content)
}

pub fn right_section(content: AnyElement) -> Div {
    div()
        .h_full()
        .flex()
        .items_center()
        .justify_end()
        .child(content)
}

pub fn bottom_bar(
    left: AnyElement,
    center: AnyElement,
    right: AnyElement,
    progress_slider: AnyElement,
) -> Div {
    div()
        .flex_none()
        .w_full()
        .h(px(page_scaffold::BOTTOM_BAR_HEIGHT_PX))
        .bg(rgba(theme::with_alpha(
            theme::COLOR_NAVBAR_BG_DARK,
            theme::ALPHA_NAVBAR_BG,
        )))
        .backdrop_blur_xl()
        .backdrop_saturation(1.8)
        .flex()
        .flex_col()
        .gap_0()
        .child(div().w_full().h(px(2.)).child(progress_slider))
        .child(
            div()
                .w_full()
                .flex_1()
                .px(relative(0.1))
                .py_0()
                .grid()
                .grid_cols(3)
                .items_center()
                .content_center()
                .child(left)
                .child(center)
                .child(right),
        )
}

pub fn render(model: &BottomBarModel, actions: &BottomBarActions) -> AnyElement {
    let icon_size_sm = 14.;
    let icon_size_md = 18.;
    let icon_color_main = theme::COLOR_TEXT_DARK;
    let mode_color = match model.mode {
        PlaybackMode::Sequence => icon_color_main,
        PlaybackMode::SingleRepeat | PlaybackMode::Shuffle => theme::COLOR_PRIMARY,
    };

    let cover = if let Some(url) = model.current_cover_url.as_ref() {
        img(image_resize_url(url, "64y64"))
            .id(format!("bottom.cover.{}", url))
            .size(px(46.))
            .rounded_md()
            .into_any_element()
    } else {
        div()
            .text_size(px(18.))
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("♪")
            .into_any_element()
    };

    let favorite_interactive = model.favorite_enabled && !model.favorite_pending;
    let left_content = div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .size(px(46.))
                .rounded_md()
                .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
                .flex()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .child(cover),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_size(px(16.))
                                .truncate()
                                .max_w_64()
                                .child(model.current_song.clone()),
                        )
                        .child(
                            div()
                                .text_color(rgb(theme::COLOR_SECONDARY))
                                .text_size(px(12.))
                                .truncate()
                                .max_w_64()
                                .child(model.current_artist.clone()),
                        ),
                )
                .children(model.has_current_song.then(|| {
                    let favorite_color = if model.favorite_liked {
                        theme::COLOR_PRIMARY
                    } else {
                        theme::COLOR_SECONDARY
                    };
                    let on_toggle_favorite = actions.on_toggle_favorite.clone();
                    button::icon_interactive(
                        "player-favorite",
                        button::icon_base(button::ButtonStyle::default())
                            .size(px(32.))
                            .text_color(rgb(favorite_color))
                            .when(!favorite_interactive, |this| {
                                this.opacity(0.45).cursor_default()
                            })
                            .when(favorite_interactive, |this| {
                                this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    cx.stop_propagation();
                                    if let Some(on_toggle_favorite) = on_toggle_favorite.as_ref() {
                                        on_toggle_favorite(cx);
                                    }
                                })
                            })
                            .child(icon::render(
                                if model.favorite_liked {
                                    IconName::HeartSolid
                                } else {
                                    IconName::Heart
                                },
                                15.,
                                favorite_color,
                            )),
                        button::ButtonStyle::default(),
                    )
                    .into_any_element()
                })),
        )
        .into_any_element();

    let prev_action = actions.on_prev.clone();
    let toggle_action = actions.on_toggle.clone();
    let next_action = actions.on_next.clone();
    let center_content = div()
        .flex()
        .items_center()
        .gap_2()
        .child(button::icon_interactive(
            "player-prev",
            button::icon_base(button::ButtonStyle::default())
                .size(px(36.))
                .text_color(rgb(icon_color_main))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| prev_action(cx))
                .child(icon::render(
                    IconName::Previous,
                    icon_size_sm,
                    icon_color_main,
                )),
            button::ButtonStyle::default(),
        ))
        .child(button::icon_interactive(
            "player-toggle",
            button::icon_base(button::ButtonStyle::default())
                .size(px(44.))
                .text_color(rgb(icon_color_main))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| toggle_action(cx))
                .child(icon::render(
                    if model.is_playing {
                        IconName::Pause
                    } else {
                        IconName::Play
                    },
                    icon_size_md,
                    icon_color_main,
                )),
            button::ButtonStyle::default(),
        ))
        .child(button::icon_interactive(
            "player-next",
            button::icon_base(button::ButtonStyle::default())
                .size(px(36.))
                .text_color(rgb(icon_color_main))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| next_action(cx))
                .child(icon::render(IconName::Next, icon_size_sm, icon_color_main)),
            button::ButtonStyle::default(),
        ))
        .into_any_element();

    let queue_action = actions.on_open_queue.clone();
    let cycle_mode_action = actions.on_cycle_mode.clone();
    let right_content = div()
        .flex()
        .items_center()
        .gap_1()
        .child(button::icon_interactive(
            "player-queue",
            button::icon_base(button::ButtonStyle::default())
                .size(px(36.))
                .text_color(rgb(icon_color_main))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| queue_action(cx))
                .child(icon::render(IconName::List, icon_size_sm, icon_color_main)),
            button::ButtonStyle::default(),
        ))
        .child(button::icon_interactive(
            "player-mode",
            button::icon_base(button::ButtonStyle::default())
                .size(px(36.))
                .text_color(rgb(mode_color))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| cycle_mode_action(cx))
                .child(icon::render(
                    mode_icon(model.mode),
                    icon_size_sm,
                    mode_color,
                )),
            button::ButtonStyle::default(),
        ))
        .child(button::icon_interactive(
            "player-volume",
            button::icon_base(button::ButtonStyle::default())
                .size(px(36.))
                .text_color(rgb(icon_color_main))
                .child(icon::render(
                    IconName::Volume,
                    icon_size_sm,
                    icon_color_main,
                )),
            button::ButtonStyle::default(),
        ))
        .child(div().w(px(120.)).child(model.volume_slider.clone()))
        .child(
            div()
                .min_w_12()
                .text_center()
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child(format!("{:.0}%", model.volume * 100.)),
        )
        .into_any_element();

    bottom_bar(
        left_section(left_content).into_any_element(),
        center_section(center_content).into_any_element(),
        right_section(right_content).into_any_element(),
        model.progress_slider.clone().into_any_element(),
    )
    .into_any_element()
}

fn mode_icon(mode: PlaybackMode) -> IconName {
    match mode {
        PlaybackMode::Shuffle => IconName::Shuffle,
        PlaybackMode::SingleRepeat => IconName::RepeatOne,
        PlaybackMode::Sequence => IconName::Repeat,
    }
}
