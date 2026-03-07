use std::ops::Range;

use gpui::{
    AnyElement, App, ElementId, IntoElement, ListSizingBehavior, ScrollStrategy, UniformList,
    UniformListScrollHandle, Window, uniform_list,
};

pub type VirtualListScrollHandle = UniformListScrollHandle;
pub type VirtualListScrollStrategy = ScrollStrategy;

pub struct VirtualList {
    inner: UniformList,
}

pub fn virtual_list<R>(
    id: impl Into<ElementId>,
    item_count: usize,
    render: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
) -> VirtualList
where
    R: IntoElement,
{
    VirtualList {
        inner: uniform_list(id, item_count, render),
    }
}

impl VirtualList {
    pub fn track_scroll(mut self, handle: &VirtualListScrollHandle) -> Self {
        self.inner = self.inner.track_scroll(handle.clone());
        self
    }

    pub fn with_sizing_behavior(mut self, behavior: ListSizingBehavior) -> Self {
        self.inner = self.inner.with_sizing_behavior(behavior);
        self
    }

    pub fn with_width_from_item(mut self, item_index: Option<usize>) -> Self {
        self.inner = self.inner.with_width_from_item(item_index);
        self
    }

    pub fn y_flipped(mut self, y_flipped: bool) -> Self {
        self.inner = self.inner.y_flipped(y_flipped);
        self
    }
}

impl IntoElement for VirtualList {
    type Element = UniformList;

    fn into_element(self) -> Self::Element {
        self.inner
    }
}

pub fn as_any(list: VirtualList) -> AnyElement {
    list.into_any_element()
}
