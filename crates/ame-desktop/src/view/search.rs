use gpui::{AnyElement, App, FontWeight, MouseButton, div, prelude::*, px, rgb};

use crate::component::{button, theme};

#[derive(Debug, Clone)]
pub struct SearchSong {
    pub id: i64,
    pub name: String,
    pub artists: String,
}

pub fn render_row(song: SearchSong, on_enqueue: impl Fn(&mut App) + 'static) -> AnyElement {
    div()
        .w_full()
        .rounded_lg()
        .bg(rgb(theme::COLOR_CARD_DARK))
        .px_3()
        .py_2()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .child(div().font_weight(FontWeight::BOLD).child(song.name))
                .child(
                    div()
                        .text_color(rgb(theme::COLOR_SECONDARY))
                        .child(song.artists),
                ),
        )
        .child(
            button::pill_base("入队").on_mouse_down(MouseButton::Left, move |_, _, cx| {
                on_enqueue(cx);
            }),
        )
        .into_any_element()
}

pub fn render(
    keyword: &str,
    loading: bool,
    error: Option<&str>,
    rows: Vec<AnyElement>,
) -> AnyElement {
    let title = if keyword.is_empty() {
        "搜索".to_string()
    } else {
        format!("搜索: {keyword}")
    };

    let status = if let Some(error) = error {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child(format!("搜索失败: {error}"))
            .into_any_element()
    } else if loading {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("搜索中...")
            .into_any_element()
    } else {
        div().into_any_element()
    };

    let results = if rows.is_empty() {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_CARD_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("暂无结果")
            .into_any_element()
    } else {
        rows.into_iter()
            .fold(div().w_full().flex().flex_col().gap_2(), |list, row| {
                list.child(row)
            })
            .into_any_element()
    };

    div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(28.))
        .gap_5()
        .child(
            div()
                .text_size(px(42.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_TEXT_DARK))
                .child(title),
        )
        .child(status)
        .child(results)
        .into_any_element()
}
