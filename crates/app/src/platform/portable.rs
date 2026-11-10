//! Pontes com Windows e Linux. O macOS tem as suas em `macos.rs`; a interface é a mesma.

pub fn clipboard_set(t: &str) {
    if let Ok(mut c) = arboard::Clipboard::new() {
        let _ = c.set_text(t.to_string());
    }
}

pub fn clipboard_get() -> String {
    arboard::Clipboard::new().and_then(|mut c| c.get_text()).unwrap_or_default()
}

pub fn pick_folder(prompt: &str) -> Option<String> {
    rfd::FileDialog::new().set_title(prompt).pick_folder().map(|p| p.to_string_lossy().into_owned())
}

/// Ainda não lemos a preferência de reduzir movimento fora do macOS: o grafo abre animado.
pub fn reduce_motion() -> bool {
    false
}

/// Segundos entre o relógio local e o UTC, agora.
#[cfg(unix)]
pub fn utc_offset() -> i64 {
    unsafe {
        let now = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&now, &mut tm).is_null() {
            return 0;
        }
        tm.tm_gmtoff as i64
    }
}

#[cfg(windows)]
pub fn utc_offset() -> i64 {
    #[repr(C)]
    struct SystemTime([u16; 8]);
    #[repr(C)]
    struct TimeZoneInformation {
        bias: i32,
        standard_name: [u16; 32],
        standard_date: SystemTime,
        standard_bias: i32,
        daylight_name: [u16; 32],
        daylight_date: SystemTime,
        daylight_bias: i32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetTimeZoneInformation(info: *mut TimeZoneInformation) -> u32;
    }
    unsafe {
        let mut tz: TimeZoneInformation = std::mem::zeroed();
        // 1 é horário padrão, 2 é horário de verão. O viés vem em minutos a oeste do UTC.
        let minutes = match GetTimeZoneInformation(&mut tz) {
            1 => tz.bias + tz.standard_bias,
            2 => tz.bias + tz.daylight_bias,
            0 => tz.bias,
            _ => 0,
        };
        -(minutes as i64) * 60
    }
}

/// Fora do macOS fechar a janela já chega ao laço como evento: não há o que pendurar.
pub fn hook_terminate(_on_quit: fn()) {}
