use nekowg::{AnyElement, div, prelude::*, px, rgb};

use crate::component::button;
use crate::component::theme;

fn chip(text: &'static str, active: bool) -> impl IntoElement {
    button::chip_base(text, active)
        .mr(px(12.))
        .mt(px(8.))
        .mb(px(4.))
        .hover(|this| {
            this.bg(rgb(theme::COLOR_PRIMARY_BG_DARK))
                .text_color(rgb(theme::COLOR_PRIMARY))
        })
}

pub(super) fn render_filter_row() -> AnyElement {
    div()
        .w_full()
        .flex()
        .flex_wrap()
        .mt(px(4.))
        .mb(px(16.))
        .child(chip("全部", true))
        .child(chip("推荐歌单", false))
        .child(chip("排行榜", false))
        .child(chip("流行", false))
        .into_any_element()
}
