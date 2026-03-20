use nekowg::{AnyElement, Context, ScrollWheelEvent, div, prelude::*, relative};

use crate::component::page_scaffold;

use super::super::RootView;

impl RootView {
    pub(super) fn render_main_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let routes = self.page_host.clone().into_any_element();

        div()
            .id("main-content")
            .size_full()
            .relative()
            .overflow_hidden()
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, window, cx| {
                this.main_scroll.apply_scroll_delta(
                    event.delta,
                    window.line_height(),
                    &this.main_scroll_config,
                );
                cx.stop_propagation();
                cx.notify();
            }))
            .px(relative(0.1))
            .py_0()
            .child(
                div()
                    .id("main-scroll-viewport")
                    .w_full()
                    .h_full()
                    .track_scroll(&self.main_scroll.handle)
                    .overflow_hidden()
                    .child(page_scaffold::overlay_scroll_content(routes)),
            )
            .child(self.render_scrollbar(cx))
            .into_any_element()
    }
}
