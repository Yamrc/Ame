use gpui::{AnyElement, App, FontWeight, MouseButton, div, prelude::*, px, relative, rgb};

use crate::component::button;
use crate::component::theme;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryPlaylistCard {
    pub id: i64,
    pub name: String,
    pub track_count: u32,
    pub creator_name: String,
    pub cover_url: Option<String>,
}

pub fn playlist_row(item: LibraryPlaylistCard, on_open: impl Fn(&mut App) + 'static) -> AnyElement {
    div()
        .w_full()
        .rounded_lg()
        .bg(rgb(theme::COLOR_CARD_DARK))
        .px_4()
        .py_3()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(18.))
                        .font_weight(FontWeight::BOLD)
                        .child(item.name),
                )
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(theme::COLOR_SECONDARY))
                        .child(format!(
                            "{} 首 · by {}",
                            item.track_count, item.creator_name
                        )),
                ),
        )
        .child(
            button::pill_base("打开").on_mouse_down(MouseButton::Left, move |_, _, cx| {
                on_open(cx);
            }),
        )
        .into_any_element()
}

pub fn render(
    title: &str,
    loading: bool,
    error: Option<&str>,
    rows: Vec<AnyElement>,
) -> AnyElement {
    let liked_song_preview = [
        ("夕日坂", "doriko, 初音ミク"),
        ("crossing field", "LiSA"),
        ("恋のEveryDay", "竹達彩奈"),
        ("カナリア", "ReoNa"),
        ("エソア", "もすももす"),
        ("君だったら", "HAPPY BIRTHDAY"),
        ("愛の残滓", "蓝月なくる"),
        ("HYDRA", "MYTH & ROID"),
        ("シリウスの心臓", "井口裕香"),
        ("さよーならまたいつか", "明透"),
        ("地球最後の告白を", "鹿乃"),
        ("夕日坂 (Acoustic)", "花たん"),
    ];

    let status = if let Some(error) = error {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child(format!("加载失败: {error}"))
            .into_any_element()
    } else if loading {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_SECONDARY_BG_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("加载中...")
            .into_any_element()
    } else {
        div().into_any_element()
    };

    let playlist_section = if rows.is_empty() {
        div()
            .w_full()
            .rounded_lg()
            .bg(rgb(theme::COLOR_CARD_DARK))
            .px_4()
            .py_3()
            .text_color(rgb(theme::COLOR_SECONDARY))
            .child("暂无歌单")
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
        .pt(px(20.))
        .child(
            div()
                .text_size(px(42.))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(theme::COLOR_TEXT_DARK))
                .child(title.to_string()),
        )
        .child(
            div()
                .w_full()
                .mt(px(20.))
                .flex()
                .items_start()
                .child(
                    div()
                        .w(relative(0.330))
                        .mt(px(8.))
                        .cursor_pointer()
                        .rounded_2xl()
                        .px(px(24.))
                        .py(px(18.))
                        .bg(rgb(theme::COLOR_PRIMARY_BG_DARK))
                        .h(px(228.))
                        .flex()
                        .flex_col()
                        .justify_end()
                        .child(
                            div()
                                .text_size(px(24.))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(theme::COLOR_PRIMARY))
                                .child("我喜欢的音乐"),
                        )
                        .child(
                            div()
                                .text_size(px(15.))
                                .mt(px(2.))
                                .text_color(rgb(theme::COLOR_PRIMARY))
                                .child("95 首歌"),
                        ),
                )
                .child(
                    div()
                        .w(relative(0.671))
                        .mt(px(8.))
                        .ml(px(36.))
                        .overflow_hidden()
                        .grid()
                        .grid_cols(3)
                        .gap(px(8.))
                        .children(liked_song_preview.into_iter().map(|(song_title, artist)| {
                            div()
                                .w_full()
                                .h(px(48.))
                                .flex()
                                .items_center()
                                .gap(px(10.))
                                .child(div().size(px(36.)).rounded_md().bg(rgb(0x3B3B3B)))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .overflow_hidden()
                                        .child(
                                            div()
                                                .text_size(px(16.))
                                                .font_weight(FontWeight::BOLD)
                                                .overflow_hidden()
                                                .child(song_title),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(13.))
                                                .text_color(rgb(theme::COLOR_SECONDARY))
                                                .overflow_hidden()
                                                .child(artist),
                                        ),
                                )
                        })),
                ),
        )
        .child(div().w_full().mt(px(20.)).child(status))
        .child(div().w_full().mt(px(12.)).child(playlist_section))
        .into_any_element()
}
