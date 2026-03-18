use nekowg::{AnyElement, IntoElement, div, prelude::*, px};

pub const TITLE_BAR_HEIGHT_PX: f32 = 32.0;
pub const NAV_BAR_HEIGHT_PX: f32 = 48.0;
pub const TOP_CHROME_HEIGHT_PX: f32 = TITLE_BAR_HEIGHT_PX + NAV_BAR_HEIGHT_PX;
pub const BOTTOM_BAR_HEIGHT_PX: f32 = 64.0;
pub const DEFAULT_CONTENT_TOP_SPACER_PX: f32 = TOP_CHROME_HEIGHT_PX - 28.0;
pub const DEFAULT_CONTENT_BOTTOM_SPACER_PX: f32 = BOTTOM_BAR_HEIGHT_PX;

pub fn overlay_scroll_content(child: impl IntoElement) -> AnyElement {
    div()
        .w_full()
        .child(div().w_full().h(px(DEFAULT_CONTENT_TOP_SPACER_PX)))
        .child(child)
        .child(div().w_full().h(px(DEFAULT_CONTENT_BOTTOM_SPACER_PX)))
        .into_any_element()
}
