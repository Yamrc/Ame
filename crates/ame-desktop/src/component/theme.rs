pub const COLOR_BODY_BG_DARK: u32 = 0x222222;
pub const COLOR_TEXT_DARK: u32 = 0xFFFFFF;
pub const COLOR_PRIMARY: u32 = 0x335EEA;
pub const COLOR_PRIMARY_BG_DARK: u32 = 0xBBCDFF;
pub const COLOR_SECONDARY: u32 = 0x7A7A7B;
pub const COLOR_SECONDARY_BG_DARK: u32 = 0x323232;
pub const COLOR_NAVBAR_BG_DARK: u32 = 0x222222;
pub const COLOR_LINE_DARK: u32 = 0x3A3A3A;
pub const COLOR_CARD_DARK: u32 = 0x2C2C2C;
pub const COLOR_SECONDARY_BG_TRANSPARENT_DARK: u32 = 0xFFFFFF;

pub const ALPHA_NAVBAR_BG: u8 = 0xDB;
pub const ALPHA_SECONDARY_BG_TRANSPARENT: u8 = 0x14;

pub const fn with_alpha(color: u32, alpha: u8) -> u32 {
    (color << 8) | alpha as u32
}
