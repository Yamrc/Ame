use nekowg::{AnyElement, FontWeight, div, prelude::*, px, rgb};

use crate::component::theme;
use crate::view::common;

pub fn render(
    loading: bool,
    error: Option<&str>,
    rows: Vec<AnyElement>,
    action: Option<AnyElement>,
) -> AnyElement {
    let status = common::status_banner(loading, error, "加载中...", "加载失败");

    let list = if rows.is_empty() {
        common::empty_card("暂无歌曲")
    } else {
        common::stacked_rows(rows, px(8.))
    };

    let header_content = div()
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(px(42.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_TEXT_DARK))
                .child("每日歌曲推荐"),
        )
        .child(
            div()
                .text_size(px(16.))
                .text_color(rgb(theme::COLOR_SECONDARY))
                .child("根据你的音乐口味生成 · 每天 6:00 更新"),
        );

    let header = if let Some(action) = action {
        div()
            .w_full()
            .flex()
            .items_end()
            .justify_between()
            .child(header_content)
            .child(action)
    } else {
        header_content
    };

    div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(28.))
        .gap_4()
        .child(header)
        .child(status)
        .child(list)
        .into_any_element()
}
