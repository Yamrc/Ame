use std::sync::Arc;

use nekowg::{AnyElement, App, FontWeight, MouseButton, ObjectFit, div, img, prelude::*, px, rgb};

use crate::component::{button, theme};
use crate::util::url::image_resize_url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistItemProps {
    pub id: i64,
    pub name: String,
    pub creator: String,
    pub track_count: Option<u32>,
    pub cover_url: Option<String>,
    pub cover_size: nekowg::Pixels,
}

#[derive(Clone)]
pub struct PlaylistItemActions {
    pub on_open: Arc<dyn Fn(&mut App)>,
}

pub fn render(props: PlaylistItemProps, actions: PlaylistItemActions) -> AnyElement {
    let subtitle = match props.track_count {
        Some(track_count) => format!("{track_count} 首 · by {}", props.creator),
        None => props.creator.clone(),
    };

    div()
        .w_full()
        .rounded_lg()
        .bg(rgb(theme::COLOR_CARD_DARK))
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.))
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .items_center()
                .gap(px(12.))
                .child(match props.cover_url.as_deref() {
                    Some(url) => img(image_resize_url(url, "256y256"))
                        .id(format!("playlist.cover.{}", url))
                        .size(props.cover_size)
                        .rounded_md()
                        .object_fit(ObjectFit::Cover)
                        .into_any_element(),
                    None => div()
                        .size(props.cover_size)
                        .rounded_md()
                        .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
                        .into_any_element(),
                })
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(18.))
                                .font_weight(FontWeight::BOLD)
                                .truncate()
                                .child(props.name),
                        )
                        .child(
                            div()
                                .text_size(px(14.))
                                .text_color(rgb(theme::COLOR_SECONDARY))
                                .truncate()
                                .child(subtitle),
                        ),
                ),
        )
        .child(
            button::pill_base("打开")
                .on_mouse_down(MouseButton::Left, move |_, _, cx| (actions.on_open)(cx)),
        )
        .into_any_element()
}
