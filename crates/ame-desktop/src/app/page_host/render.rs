use nekowg::{IntoElement, Render, Window};

use super::PageHostView;

impl Render for PageHostView {
    fn render(&mut self, _window: &mut Window, cx: &mut nekowg::Context<Self>) -> impl IntoElement {
        if self.active.is_none() {
            self.handle_route_change(cx);
        }
        if let Some(active) = self.active.as_ref() {
            active.slot.element()
        } else {
            nekowg::div().into_any_element()
        }
    }
}
