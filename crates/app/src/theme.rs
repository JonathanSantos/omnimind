//! Tokens do Omnimind: claro e escuro, tinta neutra e um acento. Ver `docs/brand`.
use gfx::{Color, TextStyle};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};

pub const SANS: &str = "Familjen Grotesk";
pub const MONO: &str = "Sometype Mono";

pub struct Theme {
    pub bg: Color,
    pub side: Color,
    pub panel: Color,
    pub ink: Color,
    pub sub: Color,
    pub mute: Color,
    pub line: Color,
    pub hover: Color,
    pub sel: Color,
    pub acc: Color,
    pub warn: Color,
    pub code_bg: Color,
    pub select: Color,
    pub shadow: Color,
    pub scrim: Color,
    /// Uma cor por fatia de tema, a mesma no grafo e nas métricas.
    pub themes: [Color; graph::SLOTS],
}

const LIGHT: Theme = Theme {
    bg: Color::hex(0xFFFFFF),
    side: Color::hex(0xF7F7F6),
    panel: Color::hex(0xFFFFFF),
    ink: Color::hex(0x18181A),
    sub: Color::hex(0x55555C),
    mute: Color::hex(0x6E6E76),
    line: Color::hex(0xE7E7E4),
    hover: Color::hex(0xF0F0EE),
    sel: Color::hex(0xEBEBE9),
    acc: Color::hex(0x1F7A5C),
    warn: Color::hex(0xB4401F),
    code_bg: Color::hex(0xF4F4F2),
    select: Color::rgba(0.12, 0.48, 0.36, 0.18),
    shadow: Color::rgba(0.08, 0.08, 0.09, 0.14),
    scrim: Color::rgba(0.06, 0.06, 0.08, 0.24),
    themes: [Color::hex(0xC9452E), Color::hex(0xB5801A), Color::hex(0x1F7A5C), Color::hex(0x1E7F96), Color::hex(0x2B5FD9), Color::hex(0xC23B73), Color::hex(0x7E6F55)],
};

const DARK: Theme = Theme {
    bg: Color::hex(0x141416),
    side: Color::hex(0x19191C),
    panel: Color::hex(0x1D1D20),
    ink: Color::hex(0xEDEDEE),
    sub: Color::hex(0xB0B0B7),
    mute: Color::hex(0x94949C),
    line: Color::hex(0x2D2D32),
    hover: Color::hex(0x232327),
    sel: Color::hex(0x2A2A2F),
    acc: Color::hex(0x4CC29A),
    warn: Color::hex(0xF2836C),
    code_bg: Color::hex(0x1D1D20),
    select: Color::rgba(0.3, 0.76, 0.6, 0.24),
    shadow: Color::rgba(0.0, 0.0, 0.0, 0.5),
    scrim: Color::rgba(0.0, 0.0, 0.0, 0.42),
    themes: [Color::hex(0xF2836C), Color::hex(0xE3B04B), Color::hex(0x4CC29A), Color::hex(0x4FB3C8), Color::hex(0x8AADF6), Color::hex(0xEE8FB0), Color::hex(0xD8CBB3)],
};

static IS_DARK: AtomicBool = AtomicBool::new(false);

pub fn set_dark(dark: bool) {
    IS_DARK.store(dark, Ordering::Relaxed);
}

pub fn is_dark() -> bool {
    IS_DARK.load(Ordering::Relaxed)
}

/// `THEME.ink` lê sempre o tema ativo: trocar de tema não pede recriar nada.
pub struct Active;

impl Deref for Active {
    type Target = Theme;
    fn deref(&self) -> &Theme {
        if is_dark() { &DARK } else { &LIGHT }
    }
}

pub static THEME: Active = Active;

pub fn body() -> TextStyle {
    TextStyle::new(SANS, 16.0, THEME.ink).leading(1.65)
}
pub fn ui(size: f32) -> TextStyle {
    TextStyle::new(SANS, size, THEME.ink).leading(1.4)
}
pub fn mono(size: f32) -> TextStyle {
    TextStyle::new(MONO, size, THEME.mute).leading(1.4)
}
