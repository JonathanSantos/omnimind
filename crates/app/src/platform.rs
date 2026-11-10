//! Pontes finas com o sistema: área de transferência, seletor de pasta, pastas do usuário, saída segura.
//! Cada sistema tem o seu arquivo, e o resto do app só conhece o que está aqui.

#[cfg(target_os = "macos")]
#[path = "platform/macos.rs"]
mod sys;
#[cfg(not(target_os = "macos"))]
#[path = "platform/portable.rs"]
mod sys;

pub use sys::{hook_terminate, pick_folder, reduce_motion, utc_offset};

use std::borrow::Cow;
use std::path::PathBuf;

/// Área de transferência do sistema, ou uma local quando não há janela.
pub struct Clipboard {
    headless: bool,
    mem: String,
}

impl Clipboard {
    pub fn system() -> Self {
        Self { headless: false, mem: String::new() }
    }
    pub fn local() -> Self {
        Self { headless: true, mem: String::new() }
    }
    pub fn set(&mut self, t: &str) {
        self.mem = t.to_string();
        if !self.headless {
            sys::clipboard_set(t);
        }
    }
    pub fn get(&mut self) -> String {
        if self.headless {
            return self.mem.clone();
        }
        sys::clipboard_get()
    }
}

pub fn home_dir() -> PathBuf {
    omnimind_core::util::home_dir().unwrap_or_else(std::env::temp_dir)
}

/// Onde o app guarda o que é dele e não do cofre: último cofre aberto e tema.
pub fn config_dir() -> PathBuf {
    let home = home_dir();
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Omnimind")
    } else if cfg!(windows) {
        home.join("AppData").join("Roaming").join("Omnimind")
    } else {
        std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()).map_or_else(|| home.join(".config"), PathBuf::from).join("omnimind")
    }
}

pub fn default_brain_path() -> String {
    home_dir().join("Omnimind").to_string_lossy().into_owned()
}

/// Atalho como a pessoa vê no teclado dela: ⌘ no macOS, Ctrl nos outros.
pub fn keys(label: &str) -> Cow<'_, str> {
    if cfg!(target_os = "macos") {
        return Cow::Borrowed(label);
    }
    Cow::Owned(label.replace("⌘ + ", "Ctrl + ").replace('⌘', "Ctrl+"))
}
