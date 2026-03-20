use nekowg::{AnyElement, Pixels, SharedString, div, prelude::*, rgb};

use crate::component::theme;

pub fn status_banner(
    loading: bool,
    error: Option<&str>,
    loading_text: impl Into<SharedString>,
    error_prefix: impl Into<SharedString>,
) -> AnyElement {
    let loading_text: SharedString = loading_text.into();
    let error_prefix: SharedString = error_prefix.into();

    if let Some(error) = error {
        return div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child(format!("{error_prefix}: {error}"))
            .into_any_element();
    }

    if loading {
        return div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child(loading_text)
            .into_any_element();
    }

    div().into_any_element()
}

pub fn empty_card(text: impl Into<SharedString>) -> AnyElement {
    let text: SharedString = text.into();
    div()
        .w_full()
        .rounded_lg()
        .bg(rgb(theme::COLOR_CARD_DARK))
        .px_4()
        .py_3()
        .text_color(rgb(theme::COLOR_SECONDARY))
        .child(text)
        .into_any_element()
}

pub fn stacked_rows(rows: Vec<AnyElement>, gap: Pixels) -> AnyElement {
    rows.into_iter()
        .fold(div().w_full().flex().flex_col().gap(gap), |list, row| {
            list.child(row)
        })
        .into_any_element()
}

pub fn grid_or_empty(
    items: Vec<AnyElement>,
    columns: usize,
    gap: Pixels,
    empty_label: impl Into<SharedString>,
) -> AnyElement {
    if items.is_empty() {
        return empty_card(empty_label);
    }

    items
        .into_iter()
        .fold(
            div().w_full().grid().grid_cols(columns as u16).gap(gap),
            |grid, item| grid.child(item),
        )
        .into_any_element()
}

pub fn grid_or_placeholder(
    items: Vec<AnyElement>,
    columns: usize,
    gap: Pixels,
    empty_label: impl Into<SharedString>,
    min_height: Pixels,
) -> AnyElement {
    if items.is_empty() {
        return div()
            .w_full()
            .min_h(min_height)
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child(empty_label.into())
            .into_any_element();
    }

    items
        .into_iter()
        .fold(
            div().w_full().grid().grid_cols(columns as u16).gap(gap),
            |grid, item| grid.child(item),
        )
        .into_any_element()
}
