use nekowg::{Context, IntoElement, Render, Window, prelude::*};

use crate::app::page::PageLifecycle;

pub(in crate::app::page_host) struct UnknownPageView {
    pub(in crate::app::page_host) path: String,
}

impl PageLifecycle for UnknownPageView {}

impl Render for UnknownPageView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        nekowg::div()
            .w_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                nekowg::div()
                    .text_size(nekowg::px(42.))
                    .font_weight(nekowg::FontWeight::BOLD)
                    .child("页面不存在"),
            )
            .child(
                nekowg::div()
                    .text_size(nekowg::px(16.))
                    .child(format!("未匹配到路由: {}", self.path)),
            )
    }
}
