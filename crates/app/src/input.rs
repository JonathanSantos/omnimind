//! Eventos do app, independentes de janela. O harness injeta exatamente os mesmos.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Enter,
    Escape,
    Backspace,
    Delete,
    Tab,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Char(char),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Key(Key, Mods),
    /// Texto confirmado, venha de tecla comum ou de composição.
    Text(String),
    /// Composição em curso: tecla morta ou IME.
    Preedit(String),
    MouseMove(f32, f32),
    MouseDown(f32, f32, u32),
    MouseUp(f32, f32),
    Scroll(f32, f32),
    Pinch(f32),
    Focus(bool),
}
