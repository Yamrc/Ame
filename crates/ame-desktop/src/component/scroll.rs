use std::sync::Arc;
use std::time::{Duration, Instant};

use nekowg::{
    AnyElement, Bounds, Context, DragMoveEvent, IsZero, MouseButton, MouseDownEvent,
    MouseMoveEvent, Pixels, Point, Render, ScrollDelta, ScrollHandle, Window, div, point,
    prelude::*, px, rgba, size,
};

use crate::component::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SmoothScrollProfile {
    LowLatency,
    Balanced,
    HighDamping,
}

#[derive(Debug, Clone, Copy)]
pub struct SmoothScrollConfig {
    pub wheel_line_multiplier: f32,
    pub damping: f32,
    pub epsilon_px: f32,
    pub tick_ms: u64,
    pub fade_delay_ms: u64,
    pub fade_duration_ms: u64,
    pub thumb_min_px: f32,
    pub track_width_px: f32,
    pub overlay_width_px: f32,
}

impl Default for SmoothScrollConfig {
    fn default() -> Self {
        Self::for_profile(SmoothScrollProfile::Balanced)
    }
}

impl SmoothScrollConfig {
    pub fn for_profile(profile: SmoothScrollProfile) -> Self {
        match profile {
            SmoothScrollProfile::LowLatency => Self {
                wheel_line_multiplier: 1.0,
                damping: 0.30,
                epsilon_px: 0.35,
                tick_ms: 8,
                fade_delay_ms: 1800,
                fade_duration_ms: 300,
                thumb_min_px: 48.0,
                track_width_px: 10.0,
                overlay_width_px: 12.0,
            },
            SmoothScrollProfile::Balanced => Self {
                wheel_line_multiplier: 1.0,
                damping: 0.22,
                epsilon_px: 0.35,
                tick_ms: 8,
                fade_delay_ms: 1800,
                fade_duration_ms: 300,
                thumb_min_px: 48.0,
                track_width_px: 10.0,
                overlay_width_px: 12.0,
            },
            SmoothScrollProfile::HighDamping => Self {
                wheel_line_multiplier: 1.0,
                damping: 0.14,
                epsilon_px: 0.35,
                tick_ms: 8,
                fade_delay_ms: 1800,
                fade_duration_ms: 300,
                thumb_min_px: 48.0,
                track_width_px: 10.0,
                overlay_width_px: 12.0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollBarStyle {
    pub overlay_width: Pixels,
    pub track_radius: Pixels,
    pub thumb_radius: Pixels,
    pub track_alpha: u8,
    pub thumb_idle_alpha: u8,
    pub thumb_hover_alpha: u8,
    pub track_color: u32,
    pub thumb_color: u32,
}

impl Default for ScrollBarStyle {
    fn default() -> Self {
        Self {
            overlay_width: px(12.),
            track_radius: px(6.),
            thumb_radius: px(6.),
            track_alpha: 0x26,
            thumb_idle_alpha: 0xA0,
            thumb_hover_alpha: 0xDC,
            track_color: theme::COLOR_SECONDARY_BG_DARK,
            thumb_color: theme::COLOR_SECONDARY,
        }
    }
}

#[allow(dead_code)]
impl ScrollBarStyle {
    pub fn overlay_width(mut self, value: Pixels) -> Self {
        self.overlay_width = value;
        self
    }

    pub fn track_radius(mut self, value: Pixels) -> Self {
        self.track_radius = value;
        self
    }

    pub fn thumb_radius(mut self, value: Pixels) -> Self {
        self.thumb_radius = value;
        self
    }

    pub fn track_alpha(mut self, value: u8) -> Self {
        self.track_alpha = value;
        self
    }

    pub fn thumb_idle_alpha(mut self, value: u8) -> Self {
        self.thumb_idle_alpha = value;
        self
    }

    pub fn thumb_hover_alpha(mut self, value: u8) -> Self {
        self.thumb_hover_alpha = value;
        self
    }

    pub fn track_color(mut self, value: u32) -> Self {
        self.track_color = value;
        self
    }

    pub fn thumb_color(mut self, value: u32) -> Self {
        self.thumb_color = value;
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThumbMetrics {
    pub track_bounds: Bounds<Pixels>,
    pub thumb_bounds: Bounds<Pixels>,
    pub max_offset_y: Pixels,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollBarModel {
    pub metrics: ThumbMetrics,
    pub opacity: f32,
    pub visible: bool,
    pub dragging: bool,
    pub hovering_bar: bool,
    pub viewport_origin: Point<Pixels>,
    pub style: ScrollBarStyle,
}

#[derive(Clone)]
pub struct ScrollBarActions<V: 'static> {
    pub on_hover: ScrollHoverCallback<V>,
    pub on_mouse_down: ScrollPointCallback<V>,
    pub on_mouse_move: ScrollPointCallback<V>,
    pub on_mouse_up: ScrollUpCallback<V>,
}

type ScrollHoverCallback<V> = Arc<dyn Fn(&mut V, bool, &mut Context<V>)>;
type ScrollPointCallback<V> = Arc<dyn Fn(&mut V, Point<Pixels>, &mut Context<V>)>;
type ScrollUpCallback<V> = Arc<dyn Fn(&mut V, &mut Context<V>)>;

#[derive(Clone, Copy)]
struct ScrollDragToken;

struct ScrollDragGhost;

impl Render for ScrollDragGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size(px(0.))
    }
}

pub fn render_scrollbar_overlay<V: 'static>(
    model: &ScrollBarModel,
    actions: &ScrollBarActions<V>,
    cx: &mut Context<V>,
) -> AnyElement {
    let track_alpha = ((model.style.track_alpha as f32) * model.opacity).round() as u8;
    let thumb_idle_alpha = ((model.style.thumb_idle_alpha as f32) * model.opacity).round() as u8;
    let thumb_hover_alpha = ((model.style.thumb_hover_alpha as f32) * model.opacity).round() as u8;
    let thumb_alpha = if model.dragging || model.hovering_bar {
        thumb_hover_alpha
    } else {
        thumb_idle_alpha
    };
    let overlay_w = model.style.overlay_width;

    let hover_action = actions.on_hover.clone();
    let mouse_down_action = actions.on_mouse_down.clone();
    let mouse_move_action = actions.on_mouse_move.clone();
    let mouse_move_action_drag = mouse_move_action.clone();
    let mouse_up_action = actions.on_mouse_up.clone();
    let mouse_up_action_out = mouse_up_action.clone();
    let viewport_origin = model.viewport_origin;
    let viewport_height = model.metrics.track_bounds.size.height;
    let metrics = model.metrics;

    div()
        .id("main-scrollbar-overlay")
        .absolute()
        .top(px(0.))
        .right(px(0.))
        .h(viewport_height)
        .w(overlay_w)
        .cursor_pointer()
        .when(!model.visible, |this| this.opacity(0.0))
        .on_hover(cx.listener(move |this, hovered, _, cx| hover_action(this, *hovered, cx)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                mouse_down_action(this, event.position - viewport_origin, cx);
                cx.stop_propagation();
            }),
        )
        .on_drag(ScrollDragToken, |_, _, _, cx| cx.new(|_| ScrollDragGhost))
        .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
            mouse_move_action(this, event.position - viewport_origin, cx);
            cx.stop_propagation();
        }))
        .on_drag_move::<ScrollDragToken>(cx.listener(
            move |this, event: &DragMoveEvent<ScrollDragToken>, _, cx| {
                mouse_move_action_drag(this, event.event.position - viewport_origin, cx);
                cx.stop_propagation();
            },
        ))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                mouse_up_action(this, cx);
                cx.stop_propagation();
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                mouse_up_action_out(this, cx);
                cx.stop_propagation();
            }),
        )
        .child(
            div()
                .absolute()
                .top(px(0.))
                .right(px(0.))
                .h(metrics.track_bounds.size.height)
                .w(overlay_w)
                .rounded(model.style.track_radius)
                .bg(rgba(theme::with_alpha(
                    model.style.track_color,
                    track_alpha,
                ))),
        )
        .child(
            div()
                .absolute()
                .top(metrics.thumb_bounds.origin.y - metrics.track_bounds.origin.y)
                .right(px(0.))
                .h(metrics.thumb_bounds.size.height)
                .w(overlay_w)
                .rounded(model.style.thumb_radius)
                .bg(rgba(theme::with_alpha(
                    model.style.thumb_color,
                    thumb_alpha,
                ))),
        )
        .into_any_element()
}

#[derive(Debug, Clone)]
pub struct SmoothScrollState {
    pub handle: ScrollHandle,
    pub target_y: Pixels,
    pub dragging: bool,
    pub drag_grab_offset_y: Pixels,
    pub hovering_bar: bool,
    pub last_interaction: Instant,
    pub is_animating: bool,
}

impl SmoothScrollState {
    pub fn new(handle: ScrollHandle) -> Self {
        Self {
            handle,
            target_y: px(0.),
            dragging: false,
            drag_grab_offset_y: px(0.),
            hovering_bar: false,
            last_interaction: Instant::now(),
            is_animating: false,
        }
    }

    pub fn mark_interaction(&mut self) {
        self.last_interaction = Instant::now();
    }

    pub fn set_hovering(&mut self, hovering: bool) {
        self.hovering_bar = hovering;
        if hovering {
            self.mark_interaction();
        }
    }

    pub fn clamp_y(&self, target: Pixels) -> Pixels {
        clamp_y_to_max_offset(target, self.handle.max_offset().y)
    }

    pub fn apply_scroll_delta(
        &mut self,
        delta: ScrollDelta,
        line_height: Pixels,
        config: &SmoothScrollConfig,
    ) {
        match delta {
            ScrollDelta::Lines(delta) => {
                // Vertical main content should only consume Y wheel input.
                self.apply_wheel_lines(delta.y, line_height, config);
            }
            ScrollDelta::Pixels(delta) => {
                // Keep touchpad/native pixel scrolling strictly vertical here.
                self.apply_wheel_pixels(delta.y);
            }
        }
    }

    pub fn apply_wheel_lines(
        &mut self,
        delta_lines: f32,
        line_height: Pixels,
        config: &SmoothScrollConfig,
    ) {
        if delta_lines == 0.0 {
            return;
        }

        self.mark_interaction();
        // Windows/GPUI native semantics: wheel-down deltas push offset more negative.
        let step = line_height * (delta_lines * config.wheel_line_multiplier);
        let current = self.handle.offset().y;
        let base = if self.is_animating {
            self.target_y
        } else {
            current
        };
        self.target_y = self.clamp_y(base + step);
        self.is_animating = true;
    }

    pub fn apply_wheel_pixels(&mut self, delta_pixels: Pixels) {
        if delta_pixels.is_zero() {
            return;
        }

        self.mark_interaction();
        let current = self.handle.offset().y;
        let next = self.clamp_y(current + delta_pixels);
        self.set_offset_y(next);
        self.target_y = next;
        self.is_animating = false;
    }

    pub fn tick(&mut self, config: &SmoothScrollConfig) -> bool {
        if self.dragging {
            self.is_animating = false;
            return false;
        }

        let current = self.handle.offset().y;
        let target = self.clamp_y(self.target_y);
        let diff = target - current;
        let epsilon = px(config.epsilon_px.max(0.01));

        if diff.abs() <= epsilon {
            if self.is_animating || current != target {
                self.set_offset_y(target);
                self.target_y = target;
                self.is_animating = false;
                return true;
            }
            return false;
        }

        let damping = config.damping.clamp(0.05, 0.45);
        let next = self.clamp_y(current + diff * damping);
        self.set_offset_y(next);
        self.target_y = target;
        self.is_animating = true;
        true
    }

    pub fn scrollbar_opacity(&self, config: &SmoothScrollConfig, now: Instant) -> f32 {
        if self.dragging || self.hovering_bar || self.is_animating {
            return 1.0;
        }

        let elapsed = now.saturating_duration_since(self.last_interaction);
        let delay = Duration::from_millis(config.fade_delay_ms);
        if elapsed <= delay {
            return 1.0;
        }

        let fade = Duration::from_millis(config.fade_duration_ms.max(1));
        let t = (elapsed - delay).as_secs_f32() / fade.as_secs_f32();
        (1.0 - t).clamp(0.0, 1.0)
    }

    pub fn thumb_metrics(&self, config: &SmoothScrollConfig) -> Option<ThumbMetrics> {
        let viewport = self.handle.bounds();
        let viewport_h = viewport.size.height;
        let viewport_w = viewport.size.width;
        let max_offset_y = self.handle.max_offset().y.max(px(0.));
        let content_h = viewport_h + max_offset_y;
        let offset_y = self.handle.offset().y;

        thumb_metrics_for(
            viewport_w,
            viewport_h,
            content_h,
            offset_y,
            px(config.thumb_min_px.max(12.0)),
            max_offset_y,
            px(config.track_width_px.max(1.0)),
        )
    }

    pub fn begin_drag_or_jump(
        &mut self,
        position: Point<Pixels>,
        config: &SmoothScrollConfig,
    ) -> bool {
        let Some(metrics) = self.thumb_metrics(config) else {
            return false;
        };

        if position.y < metrics.track_bounds.origin.y || position.y > metrics.track_bounds.bottom()
        {
            return false;
        }

        self.mark_interaction();
        let thumb_top = metrics.thumb_bounds.origin.y;
        let thumb_bottom = metrics.thumb_bounds.bottom();
        let on_thumb_y = position.y >= thumb_top && position.y <= thumb_bottom;

        if on_thumb_y {
            self.dragging = true;
            self.drag_grab_offset_y = position.y - thumb_top;
            return true;
        }

        self.jump_to_track_position(position.y, &metrics);
        true
    }

    pub fn drag_to(&mut self, position: Point<Pixels>, config: &SmoothScrollConfig) -> bool {
        if !self.dragging {
            return false;
        }

        let Some(metrics) = self.thumb_metrics(config) else {
            return false;
        };

        self.mark_interaction();
        let thumb_top = position.y - metrics.track_bounds.origin.y - self.drag_grab_offset_y;
        self.set_thumb_top(thumb_top, &metrics);
        true
    }

    pub fn end_drag(&mut self) -> bool {
        if !self.dragging {
            return false;
        }

        self.dragging = false;
        self.mark_interaction();
        true
    }

    fn jump_to_track_position(&mut self, mouse_y: Pixels, metrics: &ThumbMetrics) {
        let target_top =
            mouse_y - metrics.track_bounds.origin.y - (metrics.thumb_bounds.size.height / 2.);
        self.set_thumb_top(target_top, metrics);
    }

    fn set_thumb_top(&mut self, thumb_top: Pixels, metrics: &ThumbMetrics) {
        let track_height = metrics.track_bounds.size.height;
        let thumb_height = metrics.thumb_bounds.size.height;
        let max_thumb_top = (track_height - thumb_height).max(px(0.));
        let top = thumb_top.clamp(px(0.), max_thumb_top);
        let progress = if max_thumb_top.is_zero() {
            0.0
        } else {
            (top / max_thumb_top).clamp(0.0, 1.0)
        };

        let y = -metrics.max_offset_y * progress;
        let clamped = self.clamp_y(y);
        self.set_offset_y(clamped);
        self.target_y = clamped;
        self.is_animating = false;
    }

    fn set_offset_y(&self, y: Pixels) {
        let offset = self.handle.offset();
        self.handle.set_offset(point(offset.x, y));
    }
}

pub fn clamp_y_to_max_offset(target: Pixels, max_offset_y: Pixels) -> Pixels {
    target.clamp(-max_offset_y.max(px(0.)), px(0.))
}

pub fn thumb_metrics_for(
    viewport_w: Pixels,
    viewport_h: Pixels,
    content_h: Pixels,
    offset_y: Pixels,
    thumb_min: Pixels,
    max_offset_y: Pixels,
    track_width: Pixels,
) -> Option<ThumbMetrics> {
    if viewport_h <= px(0.) || content_h <= viewport_h {
        return None;
    }

    let ratio = (viewport_h / content_h).clamp(0.0, 1.0);
    let thumb_h = (viewport_h * ratio).max(thumb_min).min(viewport_h);
    let track_h = viewport_h;
    let max_thumb_top = (track_h - thumb_h).max(px(0.));
    let progress = if max_offset_y.is_zero() {
        0.0
    } else {
        (-offset_y / max_offset_y).clamp(0.0, 1.0)
    };
    let thumb_top = max_thumb_top * progress;

    let track_w = track_width;
    let track_origin = point(viewport_w - track_w, px(0.));
    let track_bounds = Bounds::new(track_origin, size(track_w, track_h));
    let thumb_bounds = Bounds::new(point(track_origin.x, thumb_top), size(track_w, thumb_h));

    Some(ThumbMetrics {
        track_bounds,
        thumb_bounds,
        max_offset_y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_y_stays_within_bounds() {
        let max = px(420.);
        assert_eq!(clamp_y_to_max_offset(px(12.), max), px(0.));
        assert_eq!(clamp_y_to_max_offset(px(-1000.), max), px(-420.));
        assert_eq!(clamp_y_to_max_offset(px(-80.), max), px(-80.));
    }

    #[test]
    fn thumb_metrics_hidden_when_content_not_overflowing() {
        let metrics = thumb_metrics_for(
            px(320.),
            px(600.),
            px(600.),
            px(0.),
            px(48.),
            px(0.),
            px(10.),
        );
        assert!(metrics.is_none());
    }

    #[test]
    fn thumb_metrics_has_minimum_height() {
        let metrics = thumb_metrics_for(
            px(320.),
            px(600.),
            px(6000.),
            px(-100.),
            px(48.),
            px(5400.),
            px(10.),
        )
        .expect("metrics");
        assert!(metrics.thumb_bounds.size.height >= px(48.));
    }

    #[test]
    fn opacity_fades_out_after_idle() {
        let mut state = SmoothScrollState::new(ScrollHandle::default());
        let config = SmoothScrollConfig {
            fade_delay_ms: 100,
            fade_duration_ms: 100,
            ..SmoothScrollConfig::default()
        };
        let start = Instant::now();
        state.last_interaction = start;
        assert!(state.scrollbar_opacity(&config, start + Duration::from_millis(60)) > 0.99);
        assert!(state.scrollbar_opacity(&config, start + Duration::from_millis(250)) < 0.05);
    }
}
