use gpui::{
    App, Bounds, Context, Element, ElementId, Entity, EventEmitter, GlobalElementId,
    InteractiveElement, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    PaintQuad, Pixels, Point, Render, Rgba, Style, Window, div, fill, point, prelude::*, px,
    relative, rgb, rgba, size,
};
use gpui_animation::{animation::TransitionExt, transition::general::Linear};
use std::time::Duration;

use crate::component::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SliderVariant {
    #[default]
    Default,
    ProgressLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderTrackAlign {
    Center,
    Top,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderThumbVisibility {
    Always,
    Never,
    HoverOrDrag,
    DragOnly,
}

#[derive(Debug, Clone, Copy)]
pub struct SliderStyle {
    pub track_height: Pixels,
    pub root_height: Pixels,
    pub thumb_diameter: Pixels,
    pub track_align: SliderTrackAlign,
    pub round_track: bool,
    pub round_fill: bool,
    pub thumb_visibility: SliderThumbVisibility,
    pub thumb_transition_ms: u64,
    pub track_color: Rgba,
    pub fill_color: Rgba,
    pub thumb_color: Rgba,
    pub disabled_thumb_color: Rgba,
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self::for_variant(SliderVariant::Default)
    }
}

#[allow(dead_code)]
impl SliderStyle {
    pub fn for_variant(variant: SliderVariant) -> Self {
        match variant {
            SliderVariant::Default => Self {
                track_height: px(4.),
                root_height: px(20.),
                thumb_diameter: px(12.),
                track_align: SliderTrackAlign::Center,
                round_track: true,
                round_fill: true,
                thumb_visibility: SliderThumbVisibility::HoverOrDrag,
                thumb_transition_ms: 160,
                track_color: rgba(theme::with_alpha(theme::COLOR_LINE_DARK, 0xCC)),
                fill_color: rgb(theme::COLOR_PRIMARY),
                thumb_color: rgb(theme::COLOR_PRIMARY),
                disabled_thumb_color: rgba(theme::with_alpha(theme::COLOR_SECONDARY, 0xA0)),
            },
            SliderVariant::ProgressLine => Self {
                track_height: px(2.),
                root_height: px(2.),
                thumb_diameter: px(10.),
                track_align: SliderTrackAlign::Top,
                round_track: false,
                round_fill: false,
                thumb_visibility: SliderThumbVisibility::DragOnly,
                thumb_transition_ms: 160,
                track_color: rgba(theme::with_alpha(theme::COLOR_LINE_DARK, 0xCC)),
                fill_color: rgb(theme::COLOR_PRIMARY),
                thumb_color: rgb(theme::COLOR_PRIMARY),
                disabled_thumb_color: rgba(theme::with_alpha(theme::COLOR_SECONDARY, 0xA0)),
            },
        }
    }

    pub fn track_height(mut self, value: Pixels) -> Self {
        self.track_height = value;
        self
    }

    pub fn root_height(mut self, value: Pixels) -> Self {
        self.root_height = value;
        self
    }

    pub fn thumb_diameter(mut self, value: Pixels) -> Self {
        self.thumb_diameter = value;
        self
    }

    pub fn track_align(mut self, value: SliderTrackAlign) -> Self {
        self.track_align = value;
        self
    }

    pub fn round_track(mut self, value: bool) -> Self {
        self.round_track = value;
        self
    }

    pub fn round_fill(mut self, value: bool) -> Self {
        self.round_fill = value;
        self
    }

    pub fn thumb_visibility(mut self, value: SliderThumbVisibility) -> Self {
        self.thumb_visibility = value;
        self
    }

    pub fn thumb_transition_ms(mut self, value: u64) -> Self {
        self.thumb_transition_ms = value;
        self
    }

    pub fn track_color(mut self, value: Rgba) -> Self {
        self.track_color = value;
        self
    }

    pub fn fill_color(mut self, value: Rgba) -> Self {
        self.fill_color = value;
        self
    }

    pub fn thumb_color(mut self, value: Rgba) -> Self {
        self.thumb_color = value;
        self
    }

    pub fn disabled_thumb_color(mut self, value: Rgba) -> Self {
        self.disabled_thumb_color = value;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SliderEvent {
    Change(f32),
    Commit(f32),
}

pub struct SliderState {
    min: f32,
    max: f32,
    value: f32,
    dragging: bool,
    hovering: bool,
    disabled: bool,
    style: SliderStyle,
    track_bounds: Option<Bounds<Pixels>>,
}

impl EventEmitter<SliderEvent> for SliderState {}

impl SliderState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let _ = cx;
        Self {
            min: 0.0,
            max: 1.0,
            value: 0.0,
            dragging: false,
            hovering: false,
            disabled: false,
            style: SliderStyle::default(),
            track_bounds: None,
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = if max <= min { min + 1.0 } else { max };
        self.value = self.value.clamp(self.min, self.max);
        self
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(self.min, self.max);
        self
    }

    #[allow(dead_code)]
    pub fn variant(mut self, variant: SliderVariant) -> Self {
        self.style = SliderStyle::for_variant(variant);
        self
    }

    pub fn style(mut self, style: SliderStyle) -> Self {
        self.style = style;
        self
    }

    #[allow(dead_code)]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[allow(dead_code)]
    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        let next = value.clamp(self.min, self.max);
        if (self.value - next).abs() <= f32::EPSILON {
            return;
        }
        self.value = next;
        cx.emit(SliderEvent::Change(self.value));
        cx.notify();
    }

    pub fn set_value_silent(&mut self, value: f32) {
        self.value = value.clamp(self.min, self.max);
    }

    #[allow(dead_code)]
    pub fn set_style(&mut self, style: SliderStyle, cx: &mut Context<Self>) {
        self.style = style;
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn value_now(&self) -> f32 {
        self.value
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn value_ratio(&self) -> f32 {
        let span = (self.max - self.min).max(f32::EPSILON);
        ((self.value - self.min) / span).clamp(0.0, 1.0)
    }

    fn begin_drag(&mut self, event: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.dragging = true;
        self.update_by_position(event.position, cx);
    }

    fn drag_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || !self.dragging {
            return;
        }
        self.update_by_position(event.position, cx);
    }

    fn end_drag(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || !self.dragging {
            return;
        }
        self.dragging = false;
        cx.emit(SliderEvent::Commit(self.value));
        cx.notify();
    }

    fn on_hover(&mut self, hovered: bool, _: &mut Window, cx: &mut Context<Self>) {
        if self.hovering == hovered {
            return;
        }
        self.hovering = hovered;
        cx.notify();
    }

    fn clear_hover(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !self.hovering {
            return;
        }
        self.hovering = false;
        cx.notify();
    }

    fn clear_hover_on_down_out(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.hovering {
            return;
        }
        self.hovering = false;
        cx.notify();
    }

    fn update_by_position(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(bounds) = self.track_bounds else {
            return;
        };
        if bounds.size.width <= px(0.) {
            return;
        }

        let x = (position.x - bounds.left()).clamp(px(0.), bounds.size.width);
        let ratio = (x / bounds.size.width).clamp(0.0, 1.0);
        let next = self.min + (self.max - self.min) * ratio;
        if (self.value - next).abs() <= f32::EPSILON {
            return;
        }

        self.value = next;
        cx.emit(SliderEvent::Change(self.value));
        cx.notify();
    }
}

struct SliderBoundsProbe {
    state: Entity<SliderState>,
}

impl IntoElement for SliderBoundsProbe {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SliderBoundsProbe {
    type RequestLayoutState = ();
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
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.state.update(cx, |state, _| {
            let bar_h = state.style.track_height;
            let track_top = match state.style.track_align {
                SliderTrackAlign::Center => bounds.center().y - bar_h / 2.0,
                SliderTrackAlign::Top => bounds.top(),
            };
            state.track_bounds = Some(Bounds::new(
                point(bounds.left(), track_top),
                size(bounds.size.width, bar_h),
            ));
        });
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }
}

impl Render for SliderState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let entity_id = entity.entity_id();
        let ratio = self.value_ratio();
        let thumb_visible = match self.style.thumb_visibility {
            SliderThumbVisibility::Always => true,
            SliderThumbVisibility::Never => false,
            SliderThumbVisibility::HoverOrDrag => self.dragging || self.hovering,
            SliderThumbVisibility::DragOnly => self.dragging,
        };
        let bar_h = self.style.track_height;
        let thumb_d = self.style.thumb_diameter;
        let root_h = self.style.root_height;
        let (track_top, track_mt) = match self.style.track_align {
            SliderTrackAlign::Center => (relative(0.5), -bar_h / 2.0),
            SliderTrackAlign::Top => (relative(0.0), px(0.)),
        };
        let (thumb_top, thumb_mt) = match self.style.track_align {
            SliderTrackAlign::Center => (relative(0.5), -thumb_d / 2.0),
            SliderTrackAlign::Top => (relative(0.0), -(thumb_d - bar_h) / 2.0),
        };

        let track_color = self.style.track_color;
        let fill_color = self.style.fill_color;
        let thumb_color = if self.disabled {
            self.style.disabled_thumb_color
        } else {
            self.style.thumb_color
        };

        let thumb_idle_alpha = if thumb_visible { 1.0 } else { 0.0 };
        let thumb_transition_ms = Duration::from_millis(self.style.thumb_transition_ms);

        div()
            .id(("ame-slider", entity_id))
            .relative()
            .w_full()
            .h(root_h)
            .cursor_pointer()
            .on_hover(cx.listener(|this, hovered, window, cx| this.on_hover(*hovered, window, cx)))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::begin_drag))
            .on_mouse_down_out(cx.listener(Self::clear_hover_on_down_out))
            .on_mouse_move(cx.listener(Self::drag_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::end_drag))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::end_drag))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::clear_hover))
            .child(
                div()
                    .absolute()
                    .left(px(0.))
                    .right(px(0.))
                    .top(track_top)
                    .mt(track_mt)
                    .h(bar_h)
                    .when(self.style.round_track, |this| this.rounded_full())
                    .bg(track_color),
            )
            .child(
                div()
                    .absolute()
                    .left(px(0.))
                    .top(track_top)
                    .mt(track_mt)
                    .h(bar_h)
                    .w(relative(ratio))
                    .when(self.style.round_fill, |this| this.rounded_full())
                    .bg(fill_color),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(ratio))
                    .top(thumb_top)
                    .mt(thumb_mt)
                    .ml(-thumb_d / 2.0)
                    .child(
                        div()
                            .id(("ame-slider-thumb", entity_id))
                            .with_transition(("ame-slider-thumb", entity_id))
                            .size(thumb_d)
                            .rounded_full()
                            .bg(thumb_color)
                            .opacity(thumb_idle_alpha)
                            .transition_when_else(
                                thumb_visible,
                                thumb_transition_ms,
                                Linear,
                                |this| this.opacity(1.0),
                                |this| this.opacity(0.0),
                            ),
                    ),
            )
            .child(SliderBoundsProbe { state: entity })
    }
}

#[allow(dead_code)]
pub fn slider_fill_quad(bounds: Bounds<Pixels>, color: gpui::Rgba) -> PaintQuad {
    fill(bounds, color)
}
