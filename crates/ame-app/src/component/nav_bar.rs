use std::sync::Arc;

use gpui::{
    AnyElement, App, Div, Entity, FontWeight, MouseButton, SharedString, div, img, prelude::*, px,
    relative, rgb, rgba,
};
use gpui_animation::{animation::TransitionExt, transition::general::Linear};
use std::time::Duration;

use crate::component::{
    button,
    icon::{self, IconName},
    input, theme,
};

#[derive(Debug, Clone)]
pub struct NavBarModel {
    pub pathname: SharedString,
    pub search_input: Entity<input::InputState>,
    pub avatar_asset: SharedString,
}

#[derive(Clone)]
pub struct NavBarActions {
    pub on_back: Arc<dyn Fn(&mut App)>,
    pub on_forward: Arc<dyn Fn(&mut App)>,
    pub on_home: Arc<dyn Fn(&mut App)>,
    pub on_discover: Arc<dyn Fn(&mut App)>,
    pub on_library: Arc<dyn Fn(&mut App)>,
    pub on_profile: Arc<dyn Fn(&mut App)>,
}

pub fn nav_route_button(label: impl Into<SharedString>, active: bool) -> Div {
    div()
        .mx_2()
        .px(px(10.))
        .py(px(5.))
        .rounded(px(6.))
        .bg(rgba(theme::with_alpha(theme::COLOR_BODY_BG_DARK, 0x00)))
        .text_decoration_none()
        .cursor_pointer()
        .child(
            div()
                .text_size(px(18.))
                .font_weight(FontWeight::BOLD)
                .text_color(if active {
                    rgb(theme::COLOR_PRIMARY)
                } else {
                    rgb(theme::COLOR_TEXT_DARK)
                })
                .child(label.into()),
        )
}

pub fn nav_left(back: AnyElement, forward: AnyElement) -> Div {
    div().h_full().flex().items_center().justify_start().child(
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(back)
            .child(forward),
    )
}

pub fn nav_center(home: AnyElement, discover: AnyElement, library: AnyElement) -> Div {
    div().h_full().flex().items_center().justify_center().child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(home)
            .child(discover)
            .child(library),
    )
}

pub fn nav_right(search_box: AnyElement, profile: AnyElement) -> Div {
    div().h_full().flex().items_center().justify_end().child(
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(search_box)
            .child(profile),
    )
}

pub fn nav_bar(left: AnyElement, center: AnyElement, right: AnyElement) -> Div {
    div()
        .flex_none()
        .w_full()
        .h(px(48.))
        .px(relative(0.1))
        .pb(px(1.))
        .grid()
        .grid_cols(3)
        .items_center()
        .bg(rgba(theme::with_alpha(
            theme::COLOR_NAVBAR_BG_DARK,
            theme::ALPHA_NAVBAR_BG,
        )))
        .child(left)
        .child(center)
        .child(right)
}

pub fn render(model: &NavBarModel, actions: &NavBarActions) -> AnyElement {
    let icon_size_nav = 12.;
    let icon_color_main = theme::COLOR_TEXT_DARK;
    let pathname = model.pathname.to_string();
    let home_active = is_path_active(&pathname, "/");
    let discover_active = is_path_active(&pathname, "/explore");
    let library_active = is_path_active(&pathname, "/library");

    let back_action = actions.on_back.clone();
    let back_button = button::icon_interactive(
        "nav-back",
        button::icon_base(button::ButtonStyle::default())
            .size(px(34.))
            .text_color(rgb(icon_color_main))
            .child(icon::render(
                IconName::ArrowLeft,
                icon_size_nav,
                icon_color_main,
            ))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| back_action(cx)),
        button::ButtonStyle::default(),
    )
    .into_any_element();

    let forward_action = actions.on_forward.clone();
    let forward_button = button::icon_interactive(
        "nav-forward",
        button::icon_base(button::ButtonStyle::default())
            .size(px(34.))
            .text_color(rgb(icon_color_main))
            .child(icon::render(
                IconName::ArrowRight,
                icon_size_nav,
                icon_color_main,
            ))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| forward_action(cx)),
        button::ButtonStyle::default(),
    )
    .into_any_element();

    let home_action = actions.on_home.clone();
    let home_button = nav_route_button("首页", home_active)
        .id("nav-route-home")
        .on_mouse_down(MouseButton::Left, move |_, _, cx| home_action(cx))
        .with_transition("nav-route-home")
        .transition_on_hover(Duration::from_millis(200), Linear, |hovered, this| {
            if *hovered {
                this.bg(button::hover_bg())
            } else {
                this.bg(button::transparent_bg())
            }
        })
        .into_any_element();

    let discover_action = actions.on_discover.clone();
    let discover_button = nav_route_button("发现", discover_active)
        .id("nav-route-discover")
        .on_mouse_down(MouseButton::Left, move |_, _, cx| discover_action(cx))
        .with_transition("nav-route-discover")
        .transition_on_hover(Duration::from_millis(200), Linear, |hovered, this| {
            if *hovered {
                this.bg(button::hover_bg())
            } else {
                this.bg(button::transparent_bg())
            }
        })
        .into_any_element();

    let library_action = actions.on_library.clone();
    let library_button = nav_route_button("音乐库", library_active)
        .id("nav-route-library")
        .on_mouse_down(MouseButton::Left, move |_, _, cx| library_action(cx))
        .with_transition("nav-route-library")
        .transition_on_hover(Duration::from_millis(200), Linear, |hovered, this| {
            if *hovered {
                this.bg(button::hover_bg())
            } else {
                this.bg(button::transparent_bg())
            }
        })
        .into_any_element();

    let search_box = div()
        .w(px(200.))
        .flex()
        .items_center()
        .child(model.search_input.clone())
        .into_any_element();

    let profile_action = actions.on_profile.clone();
    let avatar_button = button::icon_base(button::ButtonStyle::default())
        .size(px(30.))
        .child(
            img(model.avatar_asset.clone())
                .size(px(30.))
                .rounded_full()
                .object_fit(gpui::ObjectFit::Cover),
        )
        .on_mouse_down(MouseButton::Left, move |_, _, cx| profile_action(cx))
        .into_any_element();

    nav_bar(
        nav_left(back_button, forward_button).into_any_element(),
        nav_center(home_button, discover_button, library_button).into_any_element(),
        nav_right(search_box, avatar_button).into_any_element(),
    )
    .into_any_element()
}

fn is_path_active(pathname: &str, path: &str) -> bool {
    if path == "/" {
        return pathname == "/";
    }
    pathname.starts_with(path)
}
