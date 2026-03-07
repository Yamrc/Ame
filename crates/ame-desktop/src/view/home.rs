use gpui::{AnyElement, FontWeight, div, prelude::*, px, rgb};

use crate::component::theme;

pub fn render() -> AnyElement {
    let for_you_cards = [
        ("每日推荐", "按你的口味更新", 0x3D315C_u32),
        ("私人 FM", "继续播放你喜欢的旋律", 0x77527B_u32),
    ];
    let playlist_cards = [
        ("这里是名称", "这里是简介", 0x3A4458_u32),
        ("这里是名称", "这里是简介", 0x28374E_u32),
        ("这里是名称", "这里是简介", 0x4E4871_u32),
        ("这里是名称", "这里是简介", 0x3D4D57_u32),
        ("这里是名称", "这里是简介", 0x34344C_u32),
    ];
    // 纯属占位
    let artist_cards = ["鹿乃", "doriko", "ReoNa", "米津玄师", "Aimer", "ClariS"];

    div()
        .w_full()
        .flex()
        .flex_col()
        .pt(px(32.))
        .child(
            div()
                .w_full()
                .mb(px(48.))
                .flex()
                .flex_col()
                .gap(px(18.))
                .child(
                    div()
                        .text_size(px(28.))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(theme::COLOR_TEXT_DARK))
                        .child("For You"),
                )
                .child(div().w_full().grid().grid_cols(2).gap(px(24.)).children(
                    for_you_cards.into_iter().map(|(title, subtitle, color)| {
                        div()
                            .h(px(198.))
                            .rounded_xl()
                            .bg(rgb(color))
                            .p_6()
                            .flex()
                            .items_end()
                            .justify_between()
                            .child(div().text_2xl().font_weight(FontWeight::BOLD).child(title))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(theme::COLOR_TEXT_DARK))
                                    .child(subtitle),
                            )
                    }),
                )),
        )
        .child(
            div()
                .w_full()
                .mb(px(48.))
                .flex()
                .flex_col()
                .gap(px(18.))
                .child(
                    div()
                        .text_size(px(28.))
                        .font_weight(FontWeight::BOLD)
                        .child("推荐歌单"),
                )
                .child(div().w_full().grid().grid_cols(5).gap(px(24.)).children(
                    playlist_cards.into_iter().map(|(title, subtitle, color)| {
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(div().h(px(190.)).rounded_xl().bg(rgb(color)))
                            .child(
                                div()
                                    .text_size(px(20.))
                                    .font_weight(FontWeight::BOLD)
                                    .overflow_hidden()
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .text_color(rgb(theme::COLOR_SECONDARY))
                                    .overflow_hidden()
                                    .child(subtitle),
                            )
                    }),
                )),
        )
        .child(
            div()
                .w_full()
                .mb(px(48.))
                .flex()
                .flex_col()
                .gap(px(18.))
                .child(
                    div()
                        .text_size(px(28.))
                        .font_weight(FontWeight::BOLD)
                        .child("推荐艺人"),
                )
                .child(div().w_full().grid().grid_cols(6).gap(px(24.)).children(
                    artist_cards.into_iter().map(|name| {
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_3()
                            .child(div().size(px(120.)).rounded_full().bg(rgb(0x3B3B3B)))
                            .child(div().text_lg().font_weight(FontWeight::BOLD).child(name))
                    }),
                )),
        )
        .into_any_element()
}
