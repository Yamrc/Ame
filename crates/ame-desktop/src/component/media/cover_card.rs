use std::rc::Rc;

use nekowg::{
    AnyElement, App, FontWeight, MouseButton, ObjectFit, TextAlign, div, img, prelude::*, px,
    relative, rgb,
};

use crate::component::theme;
use crate::util::url::image_resize_url;

type CardAction = Rc<dyn Fn(&mut App)>;

#[derive(Clone, Default)]
pub struct CoverCardActions {
    pub on_open: Option<CardAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaylistCoverCardProps {
    pub title: String,
    pub subtitle: String,
    pub cover_url: Option<String>,
    pub cover_height: nekowg::Pixels,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtistCoverCardProps {
    pub name: String,
    pub cover_url: Option<String>,
}

pub fn render_playlist_card(
    props: PlaylistCoverCardProps,
    actions: CoverCardActions,
) -> AnyElement {
    let on_open = actions.on_open.clone();
    div()
        .w_full()
        .when(on_open.is_some(), |this| this.cursor_pointer())
        .when_some(on_open, |this, on_open| {
            this.on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx))
        })
        .child(match props.cover_url.as_deref() {
            Some(url) => img(image_resize_url(url, "256y256"))
                .id(format!("cover.playlist.{}", url))
                .w_full()
                .h(props.cover_height)
                .rounded_xl()
                .overflow_hidden()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            None => div()
                .w_full()
                .h(props.cover_height)
                .rounded_xl()
                .bg(rgb(0x3B3B3B))
                .into_any_element(),
        })
        .child(
            div()
                .mt(px(8.))
                .text_size(px(16.))
                .line_height(relative(1.2))
                .font_weight(FontWeight::BOLD)
                .overflow_hidden()
                .text_ellipsis()
                .line_clamp(2)
                .child(props.title),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(12.))
                .line_height(relative(1.2))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_SECONDARY))
                .overflow_hidden()
                .child(props.subtitle),
        )
        .into_any_element()
}

pub fn render_artist_card(props: ArtistCoverCardProps, actions: CoverCardActions) -> AnyElement {
    let on_open = actions.on_open.clone();
    let mut avatar = div()
        .w_full()
        .relative()
        .pb(relative(1.0))
        .rounded_full()
        .overflow_hidden();

    if let Some(url) = props.cover_url.as_deref() {
        avatar = avatar.child(
            img(image_resize_url(url, "256y256"))
                .id(format!("cover.artist.{}", url))
                .absolute()
                .left(px(0.))
                .top(px(0.))
                .right(px(0.))
                .bottom(px(0.))
                .size_full()
                .object_fit(ObjectFit::Cover)
                .rounded_full(),
        );
    } else {
        avatar = avatar.bg(rgb(0x3B3B3B));
    }

    div()
        .w_full()
        .when(on_open.is_some(), |this| this.cursor_pointer())
        .when_some(on_open, |this, on_open| {
            this.on_mouse_down(MouseButton::Left, move |_, _, cx| on_open(cx))
        })
        .child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .child(avatar)
                .child(
                    div()
                        .mt(px(12.))
                        .text_size(px(15.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_align(TextAlign::Center)
                        .overflow_hidden()
                        .child(props.name),
                ),
        )
        .into_any_element()
}
