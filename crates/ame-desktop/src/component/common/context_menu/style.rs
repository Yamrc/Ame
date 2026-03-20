use nekowg::{Pixels, SharedString, px, rgba};

use crate::component::theme;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextMenuTone {
    #[default]
    Normal,
    Accent,
    Destructive,
}

#[derive(Debug, Clone)]
pub enum ContextMenuHeader {
    Track {
        cover_url: Option<SharedString>,
        title: SharedString,
        subtitle: SharedString,
    },
    Text {
        title: SharedString,
        subtitle: Option<SharedString>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct ContextMenuStyle {
    pub min_width: Pixels,
    pub max_width: Pixels,
    pub width: Pixels,
    pub window_margin: Pixels,
    pub border_width: Pixels,
    pub padding_y: Pixels,
    pub padding_x: Pixels,
    pub item_height: Pixels,
    pub item_gap: Pixels,
    pub item_padding_x: Pixels,
    pub item_content_gap: Pixels,
    pub item_radius: Pixels,
    pub radius: Pixels,
    pub separator_height: Pixels,
    pub separator_margin: Pixels,
    pub separator_inset: Pixels,
    pub fade_ms: u64,
    pub icon_size: f32,
    pub label_font_size: Pixels,
    pub label_font_weight: nekowg::FontWeight,
    pub shortcut_font_size: Pixels,
    pub shortcut_color: u32,
    pub destructive_color: u32,
    pub accent_color: u32,
    pub header_cover_size: Pixels,
    pub header_gap: Pixels,
    pub header_padding_x: Pixels,
    pub header_padding_y: Pixels,
    pub header_title_font_size: Pixels,
    pub header_subtitle_font_size: Pixels,
    pub header_title_color: u32,
    pub header_subtitle_color: u32,
    pub background: nekowg::Rgba,
    pub border_color: nekowg::Rgba,
    pub separator_color: nekowg::Rgba,
    pub hover_background: nekowg::Rgba,
    pub text_color: u32,
    pub hover_text_color: u32,
    pub disabled_opacity: f32,
}

impl Default for ContextMenuStyle {
    fn default() -> Self {
        Self {
            min_width: px(136.),
            max_width: px(240.),
            width: px(180.),
            window_margin: px(8.),
            border_width: px(1.),
            padding_y: px(6.),
            padding_x: px(6.),
            item_height: px(34.),
            item_gap: px(2.),
            item_padding_x: px(12.),
            item_content_gap: px(8.),
            item_radius: px(8.),
            radius: px(12.),
            separator_height: px(1.),
            separator_margin: px(4.),
            separator_inset: px(10.),
            fade_ms: 72,
            icon_size: 16.0,
            label_font_size: px(14.),
            label_font_weight: nekowg::FontWeight::SEMIBOLD,
            shortcut_font_size: px(12.),
            shortcut_color: theme::COLOR_SECONDARY,
            destructive_color: 0xE06C75,
            accent_color: theme::COLOR_PRIMARY,
            header_cover_size: px(44.),
            header_gap: px(10.),
            header_padding_x: px(12.),
            header_padding_y: px(10.),
            header_title_font_size: px(15.),
            header_subtitle_font_size: px(12.),
            header_title_color: theme::COLOR_TEXT_DARK,
            header_subtitle_color: theme::COLOR_SECONDARY,
            background: rgba(theme::with_alpha(theme::COLOR_CARD_DARK, 0xE6)),
            border_color: rgba(theme::with_alpha(0xFFFFFF, 0x18)),
            separator_color: rgba(theme::with_alpha(0xFFFFFF, 0x16)),
            hover_background: rgba(theme::with_alpha(theme::COLOR_PRIMARY, 0x22)),
            text_color: theme::COLOR_TEXT_DARK,
            hover_text_color: theme::COLOR_PRIMARY,
            disabled_opacity: 0.5,
        }
    }
}

#[allow(dead_code)]
impl ContextMenuStyle {
    pub fn min_width(mut self, value: Pixels) -> Self {
        self.min_width = value;
        self
    }

    pub fn max_width(mut self, value: Pixels) -> Self {
        self.max_width = value;
        self
    }

    pub fn width(mut self, value: Pixels) -> Self {
        self.width = value;
        self
    }

    pub fn window_margin(mut self, value: Pixels) -> Self {
        self.window_margin = value;
        self
    }

    pub fn border_width(mut self, value: Pixels) -> Self {
        self.border_width = value;
        self
    }

    pub fn padding_y(mut self, value: Pixels) -> Self {
        self.padding_y = value;
        self
    }

    pub fn padding_x(mut self, value: Pixels) -> Self {
        self.padding_x = value;
        self
    }

    pub fn item_height(mut self, value: Pixels) -> Self {
        self.item_height = value;
        self
    }

    pub fn item_gap(mut self, value: Pixels) -> Self {
        self.item_gap = value;
        self
    }

    pub fn item_padding_x(mut self, value: Pixels) -> Self {
        self.item_padding_x = value;
        self
    }

    pub fn item_content_gap(mut self, value: Pixels) -> Self {
        self.item_content_gap = value;
        self
    }

    pub fn item_radius(mut self, value: Pixels) -> Self {
        self.item_radius = value;
        self
    }

    pub fn radius(mut self, value: Pixels) -> Self {
        self.radius = value;
        self
    }

    pub fn separator_height(mut self, value: Pixels) -> Self {
        self.separator_height = value;
        self
    }

    pub fn separator_margin(mut self, value: Pixels) -> Self {
        self.separator_margin = value;
        self
    }

    pub fn separator_inset(mut self, value: Pixels) -> Self {
        self.separator_inset = value;
        self
    }

    pub fn fade_ms(mut self, value: u64) -> Self {
        self.fade_ms = value;
        self
    }

    pub fn icon_size(mut self, value: f32) -> Self {
        self.icon_size = value;
        self
    }

    pub fn label_font_size(mut self, value: Pixels) -> Self {
        self.label_font_size = value;
        self
    }

    pub fn label_font_weight(mut self, value: nekowg::FontWeight) -> Self {
        self.label_font_weight = value;
        self
    }

    pub fn shortcut_font_size(mut self, value: Pixels) -> Self {
        self.shortcut_font_size = value;
        self
    }

    pub fn shortcut_color(mut self, value: u32) -> Self {
        self.shortcut_color = value;
        self
    }

    pub fn destructive_color(mut self, value: u32) -> Self {
        self.destructive_color = value;
        self
    }

    pub fn accent_color(mut self, value: u32) -> Self {
        self.accent_color = value;
        self
    }

    pub fn header_cover_size(mut self, value: Pixels) -> Self {
        self.header_cover_size = value;
        self
    }

    pub fn header_gap(mut self, value: Pixels) -> Self {
        self.header_gap = value;
        self
    }

    pub fn header_padding_x(mut self, value: Pixels) -> Self {
        self.header_padding_x = value;
        self
    }

    pub fn header_padding_y(mut self, value: Pixels) -> Self {
        self.header_padding_y = value;
        self
    }

    pub fn header_title_font_size(mut self, value: Pixels) -> Self {
        self.header_title_font_size = value;
        self
    }

    pub fn header_subtitle_font_size(mut self, value: Pixels) -> Self {
        self.header_subtitle_font_size = value;
        self
    }

    pub fn header_title_color(mut self, value: u32) -> Self {
        self.header_title_color = value;
        self
    }

    pub fn header_subtitle_color(mut self, value: u32) -> Self {
        self.header_subtitle_color = value;
        self
    }

    pub fn background(mut self, value: nekowg::Rgba) -> Self {
        self.background = value;
        self
    }

    pub fn border_color(mut self, value: nekowg::Rgba) -> Self {
        self.border_color = value;
        self
    }

    pub fn separator_color(mut self, value: nekowg::Rgba) -> Self {
        self.separator_color = value;
        self
    }

    pub fn hover_background(mut self, value: nekowg::Rgba) -> Self {
        self.hover_background = value;
        self
    }

    pub fn text_color(mut self, value: u32) -> Self {
        self.text_color = value;
        self
    }

    pub fn hover_text_color(mut self, value: u32) -> Self {
        self.hover_text_color = value;
        self
    }

    pub fn disabled_opacity(mut self, value: f32) -> Self {
        self.disabled_opacity = value;
        self
    }
}
