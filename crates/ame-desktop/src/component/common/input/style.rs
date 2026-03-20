use nekowg::{Pixels, px};

use crate::component::theme;

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
