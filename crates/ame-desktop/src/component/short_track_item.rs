use std::rc::Rc;
use std::time::Duration;

use nekowg::{
    AnyElement, App, FontWeight, MouseButton, ObjectFit, SharedString, div, img, prelude::*, px,
    rgb,
};

use crate::animation::{Linear, TransitionExt};
use crate::component::context_menu::ContextMenuExt;
use crate::component::{button, theme};
use crate::util::url::image_resize_url;

type ShortTrackAction = Rc<dyn Fn(&mut App)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortTrackItemProps {
    pub id: i64,
    pub title: String,
    pub subtitle: String,
    pub cover_url: Option<String>,
    pub height: nekowg::Pixels,
}

#[derive(Clone, Default)]
pub struct ShortTrackItemActions {
    pub on_play: Option<ShortTrackAction>,
    pub on_enqueue: Option<ShortTrackAction>,
}

pub fn render(props: ShortTrackItemProps, actions: ShortTrackItemActions) -> AnyElement {
    let on_play = actions.on_play.clone();
    let on_enqueue = actions.on_enqueue.clone();
    let row_id: SharedString = format!("short-track-item-{}", props.id).into();
    let menu_id = format!("short-track-item-menu-{}", props.id);
    let base_bg = button::transparent_bg();
    let hover_bg = button::hover_bg();
    let menu_cover_url = props.cover_url.clone();
    let menu_title = props.title.clone();
    let menu_subtitle = props.subtitle.clone();
    let row = div()
        .w_full()
        .h(props.height)
        .flex()
        .items_center()
        .gap(px(10.))
        .rounded(px(10.))
        .bg(base_bg)
        .px(px(8.))
        .py(px(4.))
        .cursor_pointer()
        .when_some(on_play, |this, on_play| {
            this.on_mouse_down(MouseButton::Left, move |event, _, cx| {
                if event.click_count >= 2 {
                    on_play(cx);
                }
            })
        })
        .child(match props.cover_url.as_deref() {
            Some(url) => img(image_resize_url(url, "64y64"))
                .id(format!("cover.short.{}", url))
                .size(px(36.))
                .rounded_md()
                .overflow_hidden()
                .object_fit(ObjectFit::Cover)
                .flex_shrink_0()
                .into_any_element(),
            None => div()
                .size(px(36.))
                .rounded_md()
                .bg(rgb(0x3B3B3B))
                .flex_shrink_0()
                .into_any_element(),
        })
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(
                    div()
                        .text_size(px(16.))
                        .font_weight(FontWeight::BOLD)
                        .overflow_hidden()
                        .truncate()
                        .child(props.title),
                )
                .child(
                    div()
                        .text_size(px(13.))
                        .text_color(rgb(theme::COLOR_SECONDARY))
                        .overflow_hidden()
                        .truncate()
                        .child(props.subtitle),
                ),
        )
        .id(row_id.clone())
        .with_transition(row_id)
        .transition_on_hover(Duration::from_millis(160), Linear, move |hovered, this| {
            if *hovered {
                this.bg(hover_bg)
            } else {
                this.bg(base_bg)
            }
        });

    row.context_menu_with_id(menu_id, move |menu, _window, _cx| {
        let mut menu = menu.track_header(
            menu_cover_url.clone(),
            menu_title.clone(),
            menu_subtitle.clone(),
        );
        if let Some(on_play) = actions.on_play.clone() {
            menu = menu.item("播放", move |_window, cx| on_play(cx));
        }
        if let Some(on_enqueue) = on_enqueue.clone() {
            menu = menu.item("入队", move |_window, cx| on_enqueue(cx));
        }
        menu
    })
    .into_any_element()
}
