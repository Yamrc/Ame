mod chrome;
mod content;
mod scrollbar;

use nekowg::{Context, Render, Window, div, prelude::*, rgb};

use crate::component::theme;

use super::RootView;

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let main_content = self.render_main_content(cx);
        let top_chrome = self.render_top_chrome(window, cx);
        let bottom_chrome = self.render_bottom_chrome(cx);

        div()
            .size_full()
            .relative()
            .bg(rgb(theme::COLOR_BODY_BG_DARK))
            .text_color(rgb(theme::COLOR_TEXT_DARK))
            .font_family("Noto Sans JP")
            .font_family("Noto Sans SC")
            .overflow_hidden()
            .child(main_content)
            .child(top_chrome)
            .child(bottom_chrome)
    }
}
