//! Base gráfica: GPU, cena instanciada e texto com atlas de glifos.
//! Nada aqui sabe o que é uma nota ou uma memória.

mod color;
mod gpu;
mod renderer;
mod scene;
mod text;

pub use color::Color;
pub use gpu::{Gpu, Offscreen, UI_FORMAT};
pub use renderer::{Globals, Renderer};
pub use scene::{Rect, RectPrim, Scene};
pub use text::{Align, Family, Layout, Span, TextEngine, TextStyle};
pub use png;
pub use wgpu;
