use std::rc::Rc;

use nekowg::{AnyElement, App, FontWeight, MouseButton, div, prelude::*, px, rgb};

use crate::component::theme;

pub fn title(
    text: impl Into<String>,
    margin_top: Option<nekowg::Pixels>,
    margin_bottom: Option<nekowg::Pixels>,
) -> AnyElement {
    let mut node = div()
        .w_full()
        .text_size(px(26.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(theme::COLOR_TEXT_DARK))
        .child(text.into());

    if let Some(margin_top) = margin_top {
        node = node.mt(margin_top);
    }
    if let Some(margin_bottom) = margin_bottom {
        node = node.mb(margin_bottom);
    }

    node.into_any_element()
}

pub type SectionMoreAction = Rc<dyn Fn(&mut App)>;

pub fn header(text: impl Into<String>, on_more: Option<SectionMoreAction>) -> AnyElement {
    let title = div()
        .text_size(px(26.))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(theme::COLOR_TEXT_DARK))
        .child(text.into());

    let action = match on_more {
        Some(on_more) => div()
            .cursor_pointer()
            .text_size(px(14.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("查看全部")
            .on_mouse_down(MouseButton::Left, move |_, _, cx| on_more(cx))
            .into_any_element(),
        None => div().into_any_element(),
    };

    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .child(title)
        .child(action)
        .into_any_element()
}

pub fn block(header: AnyElement, body: AnyElement) -> AnyElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_4()
        .child(header)
        .child(body)
        .into_any_element()
}

pub fn titled(
    text: impl Into<String>,
    on_more: Option<SectionMoreAction>,
    body: AnyElement,
) -> AnyElement {
    block(header(text, on_more), body)
}
