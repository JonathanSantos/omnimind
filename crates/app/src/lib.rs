pub mod command;
pub mod data;
pub mod demo;
pub mod fonts;
pub mod graphview;
pub mod harness;
pub mod input;
pub mod model;
pub mod platform;
#[cfg(target_os = "macos")]
pub mod probe;
/// As sondas de IME e de saída dirigem o macOS por dentro. Nos outros sistemas não existem.
#[cfg(not(target_os = "macos"))]
pub mod probe {
    pub struct Probe;
    impl Probe {
        pub fn new() -> Self {
            Probe
        }
        pub fn for_quit(_note: String) -> Self {
            Probe
        }
        pub fn step(&mut self, _app: &mut crate::shell::App, _window: &winit::window::Window) -> bool {
            eprintln!("probe: only available on macOS");
            true
        }
    }
    impl Default for Probe {
        fn default() -> Self {
            Self::new()
        }
    }
}
pub mod shell;
pub mod stats;
pub mod theme;
pub mod ui;
pub mod views;

pub use platform::{default_brain_path, pick_folder, Clipboard};

