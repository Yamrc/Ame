use std::sync::Arc;

use nekowg::{AnyElement, Image, div, img, prelude::*, px, rgb};

use crate::component::theme;

pub(super) fn render_qr_area(qr_image: Option<Arc<Image>>) -> AnyElement {
    if let Some(qr_image) = qr_image {
        div()
            .w(px(280.))
            .h(px(280.))
            .rounded_lg()
            .bg(rgb(theme::COLOR_BODY_BG_DARK))
            .p_2()
            .child(img(qr_image).w_full().h_full().rounded_lg())
            .into_any_element()
    } else {
        div()
            .w(px(280.))
            .h(px(280.))
            .rounded_lg()
            .bg(rgb(theme::COLOR_BODY_BG_DARK))
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("尚未生成二维码")
            .into_any_element()
    }
}
