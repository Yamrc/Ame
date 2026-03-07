use std::ops::Range;
use std::time::Duration;

use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, Element, ElementId, ElementInputHandler,
    Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable, GlobalElementId, KeyBinding,
    LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point,
    ShapedLine, SharedString, Style, TextRun, UTF16Selection, UnderlineStyle, Window, actions, div,
    fill, point, prelude::*, px, relative, rgb, rgba, size,
};

use crate::component::theme;

const CONTEXT: &str = "AmeInput";

actions!(
    ame_input,
    [
        Enter,
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        ShowCharacterPalette,
        Paste,
        Cut,
        Copy,
    ]
);

#[derive(Clone)]
pub enum InputEvent {
    Change(SharedString),
    Submit(SharedString),
}

#[derive(Debug, Clone, Copy)]
pub struct InputStyle {
    pub height: Pixels,
    pub padding_x: Pixels,
    pub padding_y: Pixels,
    pub radius: Pixels,
    pub border_width: Pixels,
    pub text_size: Pixels,
    pub line_height: Pixels,
    pub cursor_width: Pixels,
    pub cursor_vertical_inset: Pixels,
    pub selection_vertical_inset: Pixels,
    pub blink_interval_ms: u64,
    pub placeholder_alpha: f32,
    pub selection_alpha: u8,
    pub border_color: u32,
    pub background_color: u32,
    pub text_color: u32,
    pub cursor_color: u32,
    pub selection_color: u32,
}

impl Default for InputStyle {
    fn default() -> Self {
        Self {
            height: px(34.),
            padding_x: px(10.),
            padding_y: px(6.),
            radius: px(8.),
            border_width: px(1.),
            text_size: px(16.),
            line_height: px(22.),
            cursor_width: px(2.),
            cursor_vertical_inset: px(2.),
            selection_vertical_inset: px(2.),
            blink_interval_ms: 530,
            placeholder_alpha: 0.35,
            selection_alpha: 0x44,
            border_color: theme::COLOR_LINE_DARK,
            background_color: theme::COLOR_CARD_DARK,
            text_color: theme::COLOR_TEXT_DARK,
            cursor_color: theme::COLOR_PRIMARY,
            selection_color: theme::COLOR_PRIMARY,
        }
    }
}

#[allow(dead_code)]
impl InputStyle {
    pub fn height(mut self, value: Pixels) -> Self {
        self.height = value;
        self
    }

    pub fn padding_x(mut self, value: Pixels) -> Self {
        self.padding_x = value;
        self
    }

    pub fn padding_y(mut self, value: Pixels) -> Self {
        self.padding_y = value;
        self
    }

    pub fn radius(mut self, value: Pixels) -> Self {
        self.radius = value;
        self
    }

    pub fn border_width(mut self, value: Pixels) -> Self {
        self.border_width = value;
        self
    }

    pub fn text_size(mut self, value: Pixels) -> Self {
        self.text_size = value;
        self
    }

    pub fn line_height(mut self, value: Pixels) -> Self {
        self.line_height = value;
        self
    }

    pub fn cursor_width(mut self, value: Pixels) -> Self {
        self.cursor_width = value;
        self
    }

    pub fn cursor_vertical_inset(mut self, value: Pixels) -> Self {
        self.cursor_vertical_inset = value;
        self
    }

    pub fn selection_vertical_inset(mut self, value: Pixels) -> Self {
        self.selection_vertical_inset = value;
        self
    }

    pub fn blink_interval_ms(mut self, value: u64) -> Self {
        self.blink_interval_ms = value.max(1);
        self
    }

    pub fn placeholder_alpha(mut self, value: f32) -> Self {
        self.placeholder_alpha = value.clamp(0.0, 1.0);
        self
    }

    pub fn selection_alpha(mut self, value: u8) -> Self {
        self.selection_alpha = value;
        self
    }

    pub fn border_color(mut self, value: u32) -> Self {
        self.border_color = value;
        self
    }

    pub fn background_color(mut self, value: u32) -> Self {
        self.background_color = value;
        self
    }

    pub fn text_color(mut self, value: u32) -> Self {
        self.text_color = value;
        self
    }

    pub fn cursor_color(mut self, value: u32) -> Self {
        self.cursor_color = value;
        self
    }

    pub fn selection_color(mut self, value: u32) -> Self {
        self.selection_color = value;
        self
    }
}

pub fn init_keybindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Enter, Some(CONTEXT)),
        KeyBinding::new("backspace", Backspace, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        KeyBinding::new("left", Left, Some(CONTEXT)),
        KeyBinding::new("right", Right, Some(CONTEXT)),
        KeyBinding::new("shift-left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("shift-right", SelectRight, Some(CONTEXT)),
        KeyBinding::new("home", Home, Some(CONTEXT)),
        KeyBinding::new("end", End, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some(CONTEXT)),
    ]);
}

pub struct InputState {
    focus_handle: FocusHandle,
    text: String,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    disabled: bool,
    cursor_visible: bool,
    style: InputStyle,
}

impl EventEmitter<InputEvent> for InputState {}

impl InputState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let this = Self {
            focus_handle: cx.focus_handle(),
            text: String::new(),
            placeholder: SharedString::default(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            disabled: false,
            cursor_visible: true,
            style: InputStyle::default(),
        };

        cx.spawn(async move |this, cx| {
            loop {
                let interval_ms = match this.update(cx, |this, _| this.style.blink_interval_ms) {
                    Ok(value) => value.max(1),
                    Err(_) => break,
                };
                cx.background_executor()
                    .timer(Duration::from_millis(interval_ms))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        this.cursor_visible = !this.cursor_visible;
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        this
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[allow(dead_code)]
    pub fn style(mut self, style: InputStyle) -> Self {
        self.style = style;
        self
    }

    #[allow(dead_code)]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[allow(dead_code)]
    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.text = text.into();
        let end = self.text.len();
        self.selected_range = end..end;
        self.selection_reversed = false;
        self.marked_range = None;
        self.touch_cursor();
        cx.emit(InputEvent::Change(self.text.clone().into()));
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        self.disabled = disabled;
        if disabled {
            self.is_selecting = false;
        }
        self.touch_cursor();
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn set_style(&mut self, style: InputStyle, cx: &mut Context<Self>) {
        self.style = style;
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = cx;
        window.focus(&self.focus_handle);
    }

    fn clamp_to_char_boundary(&self, mut offset: usize) -> usize {
        offset = offset.min(self.text.len());
        while offset > 0 && !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_to_char_boundary(offset);
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        self.touch_cursor();
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_to_char_boundary(offset);
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }

        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        self.touch_cursor();
        cx.notify();
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.selection_reversed = false;
        self.selected_range = 0..self.text.len();
        self.touch_cursor();
        cx.notify();
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.move_to(self.text.len(), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace('\n', " "), window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.text[self.selected_range.clone()].to_string(),
        ));
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || self.selected_range.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.text[self.selected_range.clone()].to_string(),
        ));
        self.replace_text_in_range(None, "", window, cx);
    }

    fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        cx.emit(InputEvent::Submit(self.text.clone().into()));
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        window.focus(&self.focus_handle(cx));
        self.is_selecting = true;
        self.touch_cursor();
        let index = self.index_for_mouse_position(event.position);
        if event.modifiers.shift {
            self.select_to(index, cx);
        } else {
            self.move_to(index, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_down_out(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.focus_handle.is_focused(window) {
            window.blur();
        }
        self.is_selecting = false;
        self.cursor_visible = false;
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.disabled || !self.is_selecting {
            return;
        }
        self.select_to(self.index_for_mouse_position(event.position), cx);
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.text.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };

        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.text.len();
        }

        line.closest_index_for_x(position.x - bounds.left())
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        let offset = self.clamp_to_char_boundary(offset);
        self.text[..offset]
            .char_indices()
            .last()
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        let offset = self.clamp_to_char_boundary(offset);
        if offset >= self.text.len() {
            return self.text.len();
        }

        let mut chars = self.text[offset..].char_indices();
        let _ = chars.next();
        chars
            .next()
            .map(|(idx, _)| offset + idx)
            .unwrap_or(self.text.len())
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.text.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        self.clamp_to_char_boundary(utf8_offset)
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let offset = self.clamp_to_char_boundary(offset);
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.text.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn touch_cursor(&mut self) {
        self.cursor_visible = true;
    }
}

impl EntityInputHandler for InputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.text[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }

        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.text.replace_range(range.clone(), new_text);
        let cursor = range.start + new_text.len();
        self.selected_range = cursor..cursor;
        self.selection_reversed = false;
        self.marked_range.take();
        self.touch_cursor();
        cx.emit(InputEvent::Change(self.text.clone().into()));
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }

        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.text.replace_range(range.clone(), new_text);
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| (range.start + new_range.start)..(range.start + new_range.end))
            .unwrap_or_else(|| {
                let cursor = range.start + new_text.len();
                cursor..cursor
            });
        self.selection_reversed = false;
        self.touch_cursor();
        cx.emit(InputEvent::Change(self.text.clone().into()));
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                element_bounds.left() + last_layout.x_for_index(range.start),
                element_bounds.top(),
            ),
            point(
                element_bounds.left() + last_layout.x_for_index(range.end),
                element_bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let local = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;
        let utf8_index = last_layout.index_for_x(local.x)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

impl Focusable for InputState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

struct InputTextElement {
    input: Entity<InputState>,
}

struct InputPrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for InputTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for InputTextElement {
    type RequestLayoutState = ();
    type PrepaintState = InputPrepaintState;

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
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.text.clone();
        let selected_range = input.selected_range.clone();
        let cursor_offset = input.cursor_offset();
        let text_style = window.text_style();
        let input_style = input.style;
        let text_is_empty = content.is_empty();
        let placeholder_alpha =
            (input_style.placeholder_alpha.clamp(0.0, 1.0) * 255.0).round() as u8;

        let (display_text, text_color): (SharedString, _) = if text_is_empty {
            (
                input.placeholder.clone(),
                rgba(theme::with_alpha(input_style.text_color, placeholder_alpha)).into(),
            )
        } else {
            (content.clone().into(), text_style.color)
        };

        let base_run = TextRun {
            len: display_text.len(),
            font: text_style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if !text_is_empty {
            if let Some(marked_range) = input.marked_range.clone() {
                let mut runs = Vec::new();
                if marked_range.start > 0 {
                    runs.push(TextRun {
                        len: marked_range.start,
                        ..base_run.clone()
                    });
                }
                runs.push(TextRun {
                    len: marked_range.end.saturating_sub(marked_range.start),
                    underline: Some(UnderlineStyle {
                        color: Some(base_run.color),
                        thickness: px(1.),
                        wavy: false,
                    }),
                    ..base_run.clone()
                });
                if marked_range.end < display_text.len() {
                    runs.push(TextRun {
                        len: display_text.len() - marked_range.end,
                        ..base_run.clone()
                    });
                }
                runs
            } else {
                vec![base_run.clone()]
            }
        } else {
            vec![base_run.clone()]
        };

        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let (selection, cursor) = if selected_range.is_empty() || text_is_empty {
            let cursor_x = line.x_for_index(cursor_offset.min(content.len()));
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(
                            bounds.left() + cursor_x,
                            bounds.top() + input_style.cursor_vertical_inset,
                        ),
                        size(
                            input_style.cursor_width,
                            (bounds.bottom()
                                - bounds.top()
                                - input_style.cursor_vertical_inset * 2.0)
                                .max(px(0.)),
                        ),
                    ),
                    rgb(input_style.cursor_color),
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top() + input_style.selection_vertical_inset,
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom() - input_style.selection_vertical_inset,
                        ),
                    ),
                    rgba(theme::with_alpha(
                        input_style.selection_color,
                        input_style.selection_alpha,
                    )),
                )),
                None,
            )
        };

        InputPrepaintState {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let input_state = self.input.read(cx);
        let focus_handle = input_state.focus_handle.clone();
        let disabled = input_state.disabled;
        let cursor_visible = input_state.cursor_visible;

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }
        let line = match prepaint.line.take() {
            Some(line) => line,
            None => return,
        };

        let _ = line.paint(bounds.origin, window.line_height(), window, cx);

        if !disabled
            && cursor_visible
            && focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for InputState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity_id = cx.entity().entity_id();
        let style = self.style;
        div()
            .id(("ame-input", entity_id))
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::enter))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_down_out(cx.listener(Self::on_mouse_down_out))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .h(style.height)
            .w_full()
            .px(style.padding_x)
            .py(style.padding_y)
            .rounded(style.radius)
            .border(style.border_width)
            .border_color(rgb(style.border_color))
            .bg(rgb(style.background_color))
            .text_color(rgb(style.text_color))
            .text_size(style.text_size)
            .line_height(style.line_height)
            .child(InputTextElement { input: cx.entity() })
    }
}
