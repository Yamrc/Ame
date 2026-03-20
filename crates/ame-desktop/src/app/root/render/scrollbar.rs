use std::sync::Arc;

use nekowg::{AnyElement, Context, IntoElement, div};

use crate::component::scroll::{ScrollBarActions, ScrollBarModel, ScrollBarStyle};

use super::super::RootView;

impl RootView {
    pub(super) fn render_scrollbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(metrics) = self.main_scroll.thumb_metrics(&self.main_scroll_config) else {
            return div().into_any_element();
        };
        let opacity = self
            .main_scroll
            .scrollbar_opacity(&self.main_scroll_config, std::time::Instant::now());
        let visible = opacity > 0.001;
        let viewport_origin = self.main_scroll.handle.bounds().origin;

        crate::component::scroll::render_scrollbar_overlay(
            &ScrollBarModel {
                metrics,
                opacity,
                visible,
                dragging: self.main_scroll.dragging,
                hovering_bar: self.main_scroll.hovering_bar,
                viewport_origin,
                style: ScrollBarStyle::default()
                    .overlay_width(nekowg::px(self.main_scroll_config.overlay_width_px)),
            },
            &ScrollBarActions::<Self> {
                on_hover: Arc::new(move |this, hovered, cx| {
                    this.main_scroll.set_hovering(hovered);
                    cx.notify();
                }),
                on_mouse_down: Arc::new(move |this, local_position, cx| {
                    if this
                        .main_scroll
                        .begin_drag_or_jump(local_position, &this.main_scroll_config)
                    {
                        cx.notify();
                    }
                }),
                on_mouse_move: Arc::new(move |this, local_position, cx| {
                    if this
                        .main_scroll
                        .drag_to(local_position, &this.main_scroll_config)
                    {
                        cx.notify();
                    }
                }),
                on_mouse_up: Arc::new(move |this, cx| {
                    if this.main_scroll.end_drag() {
                        cx.notify();
                    }
                }),
            },
            cx,
        )
    }
}
