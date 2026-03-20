mod builder;
mod style;

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

use nekowg::{
    AnyElement, App, Bounds, Context, Corner, DismissEvent, Element, ElementId, Entity,
    EventEmitter, FocusHandle, Focusable, Global, GlobalElementId, Hitbox, HitboxBehavior,
    InspectorElementId, InteractiveElement, IntoElement, LayoutId, MouseButton, MouseDownEvent,
    ObjectFit, ParentElement, Pixels, Point, ScrollWheelEvent, SharedString, StyleRefinement,
    Styled, Subscription, Window, anchored, deferred, div, img, point, prelude::*, px, rgb,
};

use crate::animation::{Linear, TransitionExt};
use crate::component::icon;
use crate::util::url::image_resize_url;

pub use builder::{ContextMenuBuilder, ContextMenuItem};
pub use style::{ContextMenuHeader, ContextMenuStyle, ContextMenuTone};

type ContextMenuBuilderFn =
    Rc<dyn Fn(ContextMenuBuilder, &mut Window, &mut App) -> ContextMenuBuilder>;

#[derive(Clone)]
struct ContextMenuContent {
    header: Option<ContextMenuHeader>,
    items: Vec<ContextMenuItem>,
}

impl ContextMenuContent {
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

pub trait ContextMenuExt: ParentElement + Styled {
    #[allow(dead_code)]
    fn context_menu(
        self,
        f: impl Fn(ContextMenuBuilder, &mut Window, &mut App) -> ContextMenuBuilder + 'static,
    ) -> ContextMenu<Self>
    where
        Self: Sized,
    {
        let id = format!("context-menu-{:p}", &self as *const _);
        ContextMenu::new(id, self).menu(f)
    }

    fn context_menu_with_id(
        self,
        id: impl Into<ElementId>,
        f: impl Fn(ContextMenuBuilder, &mut Window, &mut App) -> ContextMenuBuilder + 'static,
    ) -> ContextMenu<Self>
    where
        Self: Sized,
    {
        ContextMenu::new(id, self).menu(f)
    }
}

impl<E: ParentElement + Styled> ContextMenuExt for E {}

pub struct ContextMenu<E: ParentElement + Styled + Sized> {
    id: ElementId,
    element: Option<E>,
    menu: Option<ContextMenuBuilderFn>,
    anchor: Corner,
    menu_style: ContextMenuStyle,
    _ignore_style: StyleRefinement,
}

impl<E: ParentElement + Styled> ContextMenu<E> {
    pub fn new(id: impl Into<ElementId>, element: E) -> Self {
        Self {
            id: id.into(),
            element: Some(element),
            menu: None,
            anchor: Corner::TopLeft,
            menu_style: ContextMenuStyle::default(),
            _ignore_style: StyleRefinement::default(),
        }
    }

    fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(ContextMenuBuilder, &mut Window, &mut App) -> ContextMenuBuilder + 'static,
    {
        self.menu = Some(Rc::new(builder));
        self
    }

    #[allow(dead_code)]
    pub fn anchor(mut self, anchor: Corner) -> Self {
        self.anchor = anchor;
        self
    }

    #[allow(dead_code)]
    pub fn menu_style(mut self, style: ContextMenuStyle) -> Self {
        self.menu_style = style;
        self
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut ContextMenuState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<ContextMenuState, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.flatten().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

impl<E: ParentElement + Styled> ParentElement for ContextMenu<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: ParentElement + Styled> Styled for ContextMenu<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        if let Some(element) = &mut self.element {
            element.style()
        } else {
            &mut self._ignore_style
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> IntoElement for ContextMenu<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct ContextMenuSharedState {
    menu_view: Option<Entity<ContextMenuPopup>>,
    open: bool,
    position: Point<Pixels>,
    popup_bounds: Option<Bounds<Pixels>>,
    subscription: Option<Subscription>,
}

fn clear_popup_state(state: &mut ContextMenuSharedState) {
    state.open = false;
    state.menu_view = None;
    state.popup_bounds = None;
    state.subscription = None;
}

fn clear_registry_open_menu(shared_state: &Rc<RefCell<ContextMenuSharedState>>, cx: &mut App) {
    cx.update_default_global(|registry: &mut ContextMenuRegistry, _| {
        if let Some(open) = &registry.open_menu
            && open.ptr_eq(&Rc::downgrade(shared_state))
        {
            registry.open_menu = None;
        }
    });
}

#[derive(Default)]
struct ContextMenuRegistry {
    open_menu: Option<Weak<RefCell<ContextMenuSharedState>>>,
}

impl Global for ContextMenuRegistry {}

pub struct ContextMenuState {
    element: Option<AnyElement>,
    shared_state: Rc<RefCell<ContextMenuSharedState>>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            element: None,
            shared_state: Rc::new(RefCell::new(ContextMenuSharedState {
                menu_view: None,
                open: false,
                position: point(px(0.), px(0.)),
                popup_bounds: None,
                subscription: None,
            })),
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> Element for ContextMenu<E> {
    type RequestLayoutState = ContextMenuState;
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (nekowg::LayoutId, Self::RequestLayoutState) {
        let anchor = self.anchor;
        let menu_style = self.menu_style;

        self.with_element_state(
            id.expect("context_menu requires global id"),
            window,
            cx,
            |this, state, window, cx| {
                let shared_state = state.shared_state.clone();
                let (position, open) = {
                    let state = shared_state.borrow();
                    (state.position, state.open)
                };
                let menu_view = shared_state.borrow().menu_view.clone();

                let mut menu_element = None;
                if open {
                    let has_items = menu_view
                        .as_ref()
                        .map(|menu| !menu.read(cx).is_empty())
                        .unwrap_or(false);

                    if has_items {
                        menu_element = Some(
                            deferred(
                                anchored().child(
                                    div()
                                        .w(window.bounds().size.width)
                                        .h(window.bounds().size.height)
                                        .child(
                                            anchored()
                                                .position(position)
                                                .snap_to_window_with_margin(
                                                    menu_style.window_margin,
                                                )
                                                .anchor(anchor)
                                                .when_some(menu_view, |this, menu| {
                                                    this.child(menu.clone())
                                                }),
                                        ),
                                ),
                            )
                            .with_priority(1)
                            .into_any(),
                        );
                    }
                }

                let mut element = this
                    .element
                    .take()
                    .expect("context_menu element missing")
                    .children(menu_element)
                    .into_any_element();
                let layout_id = element.request_layout(window, cx);

                (
                    layout_id,
                    ContextMenuState {
                        element: Some(element),
                        shared_state,
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: nekowg::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(element) = &mut request_layout.element {
            element.prepaint(window, cx);
        }

        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: nekowg::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(element) = &mut request_layout.element {
            element.paint(window, cx);
        }

        let builder = self.menu.clone();
        let menu_style = self.menu_style;

        self.with_element_state(
            id.expect("context_menu requires global id"),
            window,
            cx,
            |_this, state, window, _| {
                let shared_state = state.shared_state.clone();
                let shared_state_for_mouse = shared_state.clone();
                let shared_state_for_scroll = shared_state.clone();
                let hitbox = hitbox.clone();

                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if !phase.bubble()
                        || event.button != MouseButton::Right
                        || !hitbox.is_hovered(window)
                    {
                        return;
                    }

                    let Some(builder) = builder.as_ref() else {
                        return;
                    };

                    let items = builder(ContextMenuBuilder::new(), window, cx).build();
                    if items.is_empty() {
                        return;
                    }

                    let mut previous = None;
                    cx.update_default_global(|registry: &mut ContextMenuRegistry, _| {
                        previous = registry.open_menu.take();
                        registry.open_menu = Some(Rc::downgrade(&shared_state_for_mouse));
                    });

                    if let Some(previous) = previous
                        && let Some(previous) = previous.upgrade()
                        && !Rc::ptr_eq(&previous, &shared_state_for_mouse)
                    {
                        clear_popup_state(&mut previous.borrow_mut());
                    }

                    {
                        let mut state = shared_state_for_mouse.borrow_mut();
                        state.position = event.position;
                        clear_popup_state(&mut state);
                        state.open = true;
                    }

                    let menu = cx.new({
                        let shared_state = Rc::downgrade(&shared_state_for_mouse);
                        move |cx| ContextMenuPopup::new(items, menu_style, shared_state, cx)
                    });
                    menu.focus_handle(cx).focus(window, cx);
                    let subscription = window.subscribe(&menu, cx, {
                        let shared_state = shared_state_for_mouse.clone();
                        move |_, _: &DismissEvent, window, cx| {
                            clear_popup_state(&mut shared_state.borrow_mut());
                            clear_registry_open_menu(&shared_state, cx);
                            window.refresh();
                        }
                    });

                    {
                        let mut state = shared_state_for_mouse.borrow_mut();
                        state.menu_view = Some(menu);
                        state.subscription = Some(subscription);
                    }
                    window.refresh();

                    cx.stop_propagation();
                });

                window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                    if !phase.bubble() {
                        return;
                    }

                    let popup_bounds = {
                        let state = shared_state_for_scroll.borrow();
                        if !state.open {
                            return;
                        }
                        state.popup_bounds
                    };

                    let Some(popup_bounds) = popup_bounds else {
                        return;
                    };

                    if popup_bounds.contains(&event.position) {
                        cx.stop_propagation();
                        return;
                    }

                    clear_popup_state(&mut shared_state_for_scroll.borrow_mut());
                    clear_registry_open_menu(&shared_state_for_scroll, cx);
                    window.refresh();
                });
            },
        );
    }
}

struct ContextMenuPopup {
    header: Option<ContextMenuHeader>,
    items: Vec<ContextMenuItem>,
    style: ContextMenuStyle,
    hovered_index: Option<usize>,
    focus_handle: FocusHandle,
    shared_state: Weak<RefCell<ContextMenuSharedState>>,
}

impl ContextMenuPopup {
    fn new(
        content: ContextMenuContent,
        style: ContextMenuStyle,
        shared_state: Weak<RefCell<ContextMenuSharedState>>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            header: content.header,
            items: content.items,
            style,
            hovered_index: None,
            focus_handle: cx.focus_handle(),
            shared_state,
        }
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
    }

    fn dismiss_on_mouse_down_out(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss(cx);
    }

    fn set_hovered_index(&mut self, index: usize, hovered: bool, cx: &mut Context<Self>) {
        let next = if hovered { Some(index) } else { None };
        if hovered {
            if self.hovered_index != next {
                self.hovered_index = next;
                cx.notify();
            }
        } else if self.hovered_index == Some(index) {
            self.hovered_index = None;
            cx.notify();
        }
    }
}

impl EventEmitter<DismissEvent> for ContextMenuPopup {}

impl Focusable for ContextMenuPopup {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

struct ContextMenuSurface {
    element: Option<AnyElement>,
    shared_state: Weak<RefCell<ContextMenuSharedState>>,
}

impl ContextMenuSurface {
    fn new(shared_state: Weak<RefCell<ContextMenuSharedState>>, element: AnyElement) -> Self {
        Self {
            element: Some(element),
            shared_state,
        }
    }
}

impl IntoElement for ContextMenuSurface {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ContextMenuSurface {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut element = self
            .element
            .take()
            .expect("context menu surface element missing");
        let layout_id = element.request_layout(window, cx);
        (layout_id, element)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(shared_state) = self.shared_state.upgrade() {
            shared_state.borrow_mut().popup_bounds = Some(bounds);
        }
        request_layout.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        request_layout.paint(window, cx);
    }
}

impl Render for ContextMenuPopup {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = cx.entity();
        let entity_id = menu.entity_id();
        let style = self.style;
        let has_header = self.header.is_some();

        let item_rows = self
            .items
            .iter()
            .cloned()
            .enumerate()
            .map(|(ix, item)| match item {
                ContextMenuItem::Separator => div()
                    .h(style.separator_height)
                    .mt(style.separator_margin)
                    .mb(style.separator_margin)
                    .mx(style.separator_inset)
                    .bg(style.separator_color)
                    .into_any_element(),
                ContextMenuItem::Item {
                    label,
                    icon,
                    shortcut,
                    tone,
                    disabled,
                    action,
                } => {
                    let is_hovered = self.hovered_index == Some(ix);
                    let base_text_color = match tone {
                        ContextMenuTone::Normal => style.text_color,
                        ContextMenuTone::Accent => style.accent_color,
                        ContextMenuTone::Destructive => style.destructive_color,
                    };
                    let icon_color = if is_hovered {
                        style.hover_text_color
                    } else {
                        base_text_color
                    };
                    let mut row = div()
                        .id(format!("ame-context-menu-item-{entity_id:?}-{ix}"))
                        .w_full()
                        .h(style.item_height)
                        .px(style.item_padding_x)
                        .rounded(style.item_radius)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(style.item_content_gap)
                        .text_size(style.label_font_size)
                        .font_weight(style.label_font_weight)
                        .text_color(rgb(base_text_color))
                        .when(is_hovered, |this| {
                            this.bg(style.hover_background)
                                .text_color(rgb(style.hover_text_color))
                        });

                    let mut leading = div().flex().items_center().gap(style.item_content_gap);
                    if let Some(icon) = icon {
                        leading = leading.child(icon::render(icon, style.icon_size, icon_color));
                    }
                    leading = leading.child(div().child(label));
                    row = row.child(leading);

                    if let Some(shortcut) = shortcut {
                        row = row.child(
                            div()
                                .text_size(style.shortcut_font_size)
                                .font_weight(nekowg::FontWeight::MEDIUM)
                                .text_color(rgb(style.shortcut_color))
                                .child(shortcut),
                        );
                    }

                    if disabled {
                        row = row.opacity(style.disabled_opacity).cursor_default();
                    } else {
                        let menu = menu.clone();
                        let action = action.clone();
                        row = row
                            .cursor_pointer()
                            .on_hover(cx.listener(move |this, hovered, _, cx| {
                                this.set_hovered_index(ix, *hovered, cx);
                            }))
                            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                cx.stop_propagation();
                                if let Some(action) = action.as_ref() {
                                    action(window, cx);
                                }
                                menu.update(cx, |menu, cx| menu.dismiss(cx));
                            });
                    }

                    row.into_any_element()
                }
            })
            .collect::<Vec<_>>();

        let header = self.header.clone().map(|header| match header {
            ContextMenuHeader::Track {
                cover_url,
                title,
                subtitle,
            } => render_context_menu_header_track(&style, cover_url, title, subtitle),
            ContextMenuHeader::Text { title, subtitle } => {
                render_context_menu_header_text(&style, title, subtitle)
            }
        });

        ContextMenuSurface::new(
            self.shared_state.clone(),
            div()
                .id(("ame-context-menu", entity_id))
                .min_w(style.min_width)
                .max_w(style.max_width)
                .w(style.width)
                .rounded(style.radius)
                .border(style.border_width)
                .border_color(style.border_color)
                .bg(style.background)
                .flex()
                .flex_col()
                .on_mouse_down_out(cx.listener(Self::dismiss_on_mouse_down_out))
                .on_scroll_wheel(|_, _, cx| {
                    cx.stop_propagation();
                })
                .with_transition(("ame-context-menu-fade", entity_id))
                .opacity(0.0)
                .transition_when_else(
                    true,
                    Duration::from_millis(style.fade_ms),
                    Linear,
                    |this| this.opacity(1.0),
                    |this| this.opacity(0.0),
                )
                .when_some(header, |this, header| this.child(header))
                .when(has_header, |this| {
                    this.child(
                        div()
                            .h(style.separator_height)
                            .mx(style.separator_inset)
                            .bg(style.separator_color),
                    )
                })
                .child(
                    div()
                        .px(style.padding_x)
                        .py(style.padding_y)
                        .flex()
                        .flex_col()
                        .gap(style.item_gap)
                        .children(item_rows),
                )
                .into_any_element(),
        )
    }
}

fn render_context_menu_header_track(
    style: &ContextMenuStyle,
    cover_url: Option<SharedString>,
    title: SharedString,
    subtitle: SharedString,
) -> AnyElement {
    let cover = match cover_url.as_deref() {
        Some(url) => img(image_resize_url(url, "96y96"))
            .id(format!("ctx.song.cover.{:?}", url))
            .size(style.header_cover_size)
            .rounded_md()
            .object_fit(ObjectFit::Cover)
            .into_any_element(),
        None => div()
            .size(style.header_cover_size)
            .rounded_md()
            .bg(rgb(0x3B3B3B))
            .into_any_element(),
    };

    div()
        .px(style.header_padding_x)
        .py(style.header_padding_y)
        .flex()
        .items_center()
        .gap(style.header_gap)
        .child(cover)
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(style.header_title_font_size)
                        .font_weight(nekowg::FontWeight::BOLD)
                        .text_color(rgb(style.header_title_color))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(title),
                )
                .child(
                    div()
                        .mt(px(2.))
                        .text_size(style.header_subtitle_font_size)
                        .text_color(rgb(style.header_subtitle_color))
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(subtitle),
                ),
        )
        .into_any_element()
}

fn render_context_menu_header_text(
    style: &ContextMenuStyle,
    title: SharedString,
    subtitle: Option<SharedString>,
) -> AnyElement {
    div()
        .px(style.header_padding_x)
        .py(style.header_padding_y)
        .flex()
        .flex_col()
        .child(
            div()
                .text_size(style.header_title_font_size)
                .font_weight(nekowg::FontWeight::BOLD)
                .text_color(rgb(style.header_title_color))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(title),
        )
        .when_some(subtitle, |this, subtitle| {
            this.child(
                div()
                    .mt(px(2.))
                    .text_size(style.header_subtitle_font_size)
                    .text_color(rgb(style.header_subtitle_color))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(subtitle),
            )
        })
        .into_any_element()
}
