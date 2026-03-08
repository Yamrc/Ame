use std::{
    cmp::{self, Ordering},
    ops::Range,
    sync::Arc,
};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, ContentMask, Div, Element, ElementId, GlobalElementId,
    Hitbox, InteractiveElement, IntoElement, LayoutId, ListSizingBehavior, Pixels, ScrollHandle,
    Stateful, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, point, px, size,
};

type RenderItems =
    dyn for<'a> Fn(Range<usize>, &'a mut Window, &'a mut App) -> Vec<AnyElement> + 'static;

#[derive(Clone)]
struct LayoutCache {
    source_heights: Arc<Vec<Pixels>>,
    heights: Vec<Pixels>,
    origins: Vec<Pixels>,
    content_height: Pixels,
}

impl Default for LayoutCache {
    fn default() -> Self {
        Self {
            source_heights: Arc::new(Vec::new()),
            heights: Vec::new(),
            origins: Vec::new(),
            content_height: px(0.),
        }
    }
}

pub struct VirtualListLayoutState {
    items: Vec<AnyElement>,
    cache: LayoutCache,
}

pub struct VirtualList {
    id: ElementId,
    base: Stateful<Div>,
    scroll_handle: ScrollHandle,
    external_viewport_handle: Option<ScrollHandle>,
    item_heights: Arc<Vec<Pixels>>,
    render_items: Box<RenderItems>,
    sizing_behavior: ListSizingBehavior,
    overscan_items: usize,
}

pub fn v_virtual_list<R>(
    id: impl Into<ElementId>,
    item_heights: Arc<Vec<Pixels>>,
    render: impl 'static + Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>,
) -> VirtualList
where
    R: IntoElement,
{
    let id: ElementId = id.into();
    let scroll_handle = ScrollHandle::default();
    let render_items = move |range: Range<usize>, window: &mut Window, cx: &mut App| {
        render(range, window, cx)
            .into_iter()
            .map(|item| item.into_any_element())
            .collect::<Vec<_>>()
    };

    VirtualList {
        id: id.clone(),
        base: div().id(id).overflow_scroll().track_scroll(&scroll_handle),
        scroll_handle,
        external_viewport_handle: None,
        item_heights,
        render_items: Box::new(render_items),
        sizing_behavior: ListSizingBehavior::default(),
        overscan_items: 2,
    }
}

impl Styled for VirtualList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl VirtualList {
    pub fn with_external_viewport_scroll(mut self, handle: &ScrollHandle) -> Self {
        self.external_viewport_handle = Some(handle.clone());
        self.base = div().id(self.id.clone());
        self
    }

    pub fn with_sizing_behavior(mut self, behavior: ListSizingBehavior) -> Self {
        self.sizing_behavior = behavior;
        self
    }

    pub fn with_overscan(mut self, items: usize) -> Self {
        self.overscan_items = items;
        self
    }
}

fn clamp_scroll_y(y: Pixels, content_height: Pixels, viewport_height: Pixels) -> Pixels {
    let max_offset = (content_height - viewport_height).max(px(0.));
    y.clamp(-max_offset, px(0.))
}

fn first_visible_index(cache: &LayoutCache, viewport_start: Pixels) -> usize {
    let mut left = 0usize;
    let mut right = cache.heights.len();
    while left < right {
        let mid = left + (right - left) / 2;
        let end = cache.origins[mid] + cache.heights[mid];
        match end.partial_cmp(&viewport_start).unwrap_or(Ordering::Less) {
            Ordering::Greater => right = mid,
            _ => left = mid + 1,
        }
    }
    left
}

fn last_visible_index_exclusive(cache: &LayoutCache, viewport_end: Pixels) -> usize {
    let mut left = 0usize;
    let mut right = cache.origins.len();
    while left < right {
        let mid = left + (right - left) / 2;
        match cache.origins[mid]
            .partial_cmp(&viewport_end)
            .unwrap_or(Ordering::Greater)
        {
            Ordering::Less => left = mid + 1,
            _ => right = mid,
        }
    }
    left
}

impl IntoElement for VirtualList {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for VirtualList {
    type RequestLayoutState = VirtualListLayoutState;
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut cache = LayoutCache::default();
        let item_count = self.item_heights.len();

        let layout_id = self.base.interactivity().request_layout(
            global_id,
            inspector_id,
            window,
            cx,
            |style, window, cx| {
                cache = window.with_element_state(
                    global_id.expect("virtual_list requires global_id"),
                    |state: Option<LayoutCache>, _| {
                        let mut state = state.unwrap_or_default();
                        if !Arc::ptr_eq(&state.source_heights, &self.item_heights) {
                            state.source_heights = self.item_heights.clone();
                            state.heights.clear();
                            state.origins.clear();
                            state.heights.reserve(item_count);
                            state.origins.reserve(item_count);

                            let mut cumulative = px(0.);
                            for height in self.item_heights.iter().take(item_count) {
                                let h = (*height).max(px(0.));
                                state.origins.push(cumulative);
                                state.heights.push(h);
                                cumulative += h;
                            }
                            state.content_height = cumulative;
                        }
                        (state.clone(), state)
                    },
                );

                match self.sizing_behavior {
                    ListSizingBehavior::Auto => window.request_layout(style, None, cx),
                    ListSizingBehavior::Infer => {
                        let content_height = cache.content_height;
                        window.request_measured_layout(style, move |known, available, _, _| {
                            let width = known.width.unwrap_or(match available.width {
                                AvailableSpace::Definite(value) => value,
                                AvailableSpace::MinContent | AvailableSpace::MaxContent => px(0.),
                            });
                            let height = known.height.unwrap_or(match available.height {
                                AvailableSpace::Definite(value) => value,
                                AvailableSpace::MinContent | AvailableSpace::MaxContent => {
                                    content_height
                                }
                            });
                            size(width, height)
                        })
                    }
                }
            },
        );

        (
            layout_id,
            VirtualListLayoutState {
                items: Vec::new(),
                cache,
            },
        )
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let style = self
            .base
            .interactivity()
            .compute_style(global_id, None, window, cx);
        let border_widths = style.border_widths.to_pixels(window.rem_size());
        let paddings = style
            .padding
            .to_pixels(bounds.size.into(), window.rem_size());

        let content_bounds = Bounds::from_corners(
            bounds.origin
                + point(
                    border_widths.left + paddings.left,
                    border_widths.top + paddings.top,
                ),
            bounds.bottom_right()
                - point(
                    border_widths.right + paddings.right,
                    border_widths.bottom + paddings.bottom,
                ),
        );

        let content_size = size(
            content_bounds.size.width.max(px(0.)),
            request_layout.cache.content_height.max(px(0.)),
        );
        let item_count = self.item_heights.len();
        let overscan = self.overscan_items;
        let external_viewport_bounds = self.external_viewport_handle.as_ref().map(|h| h.bounds());

        self.base.interactivity().prepaint(
            global_id,
            inspector_id,
            bounds,
            content_size,
            window,
            cx,
            |_, _, hitbox, window, cx| {
                request_layout.items.clear();
                if request_layout.cache.heights.is_empty() {
                    return hitbox;
                }

                let (scroll_offset, viewport_start, viewport_end, content_mask_bounds) =
                    if let Some(viewport_bounds) = external_viewport_bounds {
                        (
                            point(px(0.), px(0.)),
                            (viewport_bounds.top() - content_bounds.top()).max(px(0.)),
                            (viewport_bounds.bottom() - content_bounds.top())
                                .min(request_layout.cache.content_height),
                            bounds.intersect(&viewport_bounds),
                        )
                    } else {
                        let mut scroll_offset = self.scroll_handle.offset();
                        let clamped_y = clamp_scroll_y(
                            scroll_offset.y,
                            request_layout.cache.content_height,
                            content_bounds.size.height,
                        );
                        if clamped_y != scroll_offset.y {
                            scroll_offset.y = clamped_y;
                            self.scroll_handle.set_offset(scroll_offset);
                        }
                        (
                            scroll_offset,
                            -scroll_offset.y,
                            -scroll_offset.y + content_bounds.size.height,
                            bounds,
                        )
                    };
                if viewport_end <= viewport_start {
                    return hitbox;
                }

                let mut first = first_visible_index(&request_layout.cache, viewport_start);
                let mut last = last_visible_index_exclusive(&request_layout.cache, viewport_end);
                first = first.saturating_sub(overscan);
                last = cmp::min(last.saturating_add(overscan), item_count);
                if first >= last {
                    return hitbox;
                }

                let visible_range = first..last;
                let items = (self.render_items)(visible_range.clone(), window, cx);
                let content_mask = ContentMask {
                    bounds: content_mask_bounds,
                };

                window.with_content_mask(Some(content_mask), |window| {
                    for (mut item, item_index) in items.into_iter().zip(visible_range.clone()) {
                        let top = request_layout.cache.origins[item_index] + scroll_offset.y;
                        let item_origin = content_bounds.origin + point(scroll_offset.x, top);
                        let item_height = request_layout.cache.heights[item_index];
                        let available_space = size(
                            AvailableSpace::Definite(content_bounds.size.width),
                            AvailableSpace::Definite(item_height),
                        );
                        item.layout_as_root(available_space, window, cx);
                        item.prepaint_at(item_origin, window, cx);
                        request_layout.items.push(item);
                    }
                });

                hitbox
            },
        )
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.base.interactivity().paint(
            global_id,
            inspector_id,
            bounds,
            prepaint.as_ref(),
            window,
            cx,
            |_, window, cx| {
                for item in &mut request_layout.items {
                    item.paint(window, cx);
                }
            },
        );
    }
}
