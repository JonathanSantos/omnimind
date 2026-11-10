//! Prova de acentos no app real: injeta `CGEvent` pelo próprio `NSApp` e confere a nota.

use crate::input::{Event, Key, Mods};
use crate::shell::App;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send, msg_send_id};
use std::ffi::c_void;
use std::time::{Duration, Instant};
use winit::window::Window;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut c_void;
    fn TISGetInputSourceProperty(s: *mut c_void, k: *const c_void) -> *mut c_void;
    static kTISPropertyUnicodeKeyLayoutData: *const c_void;
    fn LMGetKbdType() -> u8;
    fn UCKeyTranslate(l: *const c_void, code: u16, action: u16, mods: u32, kbd: u32, opts: u32, dead: *mut u32, max: usize, len: *mut usize, out: *mut u16) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFDataGetBytePtr(d: *mut c_void) -> *const c_void;
}
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventCreateKeyboardEvent(src: *const c_void, k: u16, down: bool) -> *mut c_void;
    fn CGEventSetFlags(e: *mut c_void, f: u64);
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct CgRef(*mut c_void);
unsafe impl objc2::Encode for CgRef {
    const ENCODING: objc2::Encoding = objc2::Encoding::Pointer(&objc2::Encoding::Struct("__CGEvent", &[]));
}

fn translate(code: u16, shift: bool, dead: &mut u32) -> String {
    unsafe {
        let src = TISCopyCurrentKeyboardLayoutInputSource();
        let layout = CFDataGetBytePtr(TISGetInputSourceProperty(src, kTISPropertyUnicodeKeyLayoutData));
        let mut o = [0u16; 8];
        let mut n = 0;
        UCKeyTranslate(layout, code, 0, if shift { 2 } else { 0 }, LMGetKbdType() as u32, 0, dead, 8, &mut n, o.as_mut_ptr());
        String::from_utf16_lossy(&o[..n])
    }
}

fn find(ch: &str) -> Option<(u16, bool)> {
    for c in 0..51u16 {
        for s in [false, true] {
            let mut d = 0;
            if translate(c, s, &mut d) == ch && d == 0 {
                return Some((c, s));
            }
        }
    }
    None
}

/// Tecla morta cujo próximo espaço produz `shown`.
fn dead(shown: &str) -> Option<(u16, bool)> {
    for c in 0..51u16 {
        for s in [false, true] {
            let mut d = 0;
            translate(c, s, &mut d);
            if d != 0 && translate(49, false, &mut d).contains(shown) {
                return Some((c, s));
            }
        }
    }
    None
}

unsafe fn press(code: u16, shift: bool, down: bool) {
    let cg = CGEventCreateKeyboardEvent(std::ptr::null(), code, down);
    CGEventSetFlags(cg, if shift { 0x20000 } else { 0 });
    let e: Option<Retained<AnyObject>> = msg_send_id![class!(NSEvent), eventWithCGEvent: CgRef(cg)];
    let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
    let _: () = msg_send![app, sendEvent: &*e.expect("event")];
}

pub struct Probe {
    /// Sem nota: prova de acentos. Com nota: digita e dispara `terminate:` na hora.
    quit_note: Option<String>,
    script: Vec<(u16, bool)>,
    expected: String,
    step: usize,
    next: Instant,
    opened: bool,
    typed: bool,
}

impl Probe {
    pub fn for_quit(note: String) -> Self {
        let k = |c: &str| find(c).expect("key");
        let script = vec![k("f"), k("i"), k("m"), k("q"), k("u"), k("i"), k("t")];
        Self { quit_note: Some(note), script, expected: String::new(), step: 0, next: Instant::now() + Duration::from_millis(1200), opened: false, typed: false }
    }

    pub fn new() -> Self {
        let acute = dead("'").or_else(|| dead("´")).expect("dead acute");
        let tilde = dead("~").expect("dead tilde");
        let k = |c: &str| find(c).expect("key");
        let (a, c, e, o, sp) = (k("a"), k("c"), k("e"), k("o"), k(" "));
        // "coração ação é ótimo": ' + c = ç e ~ + a = ã no Brazilian Pro; ´ em outros layouts.
        let t = |c: &str| k(c);
        let script = vec![c, o, t("r"), a, acute, c, tilde, a, o, sp, a, acute, c, tilde, a, o, sp, acute, e, sp, acute, o, t("t"), t("i"), t("m"), o];
        Self { quit_note: None, script, expected: "coração ação é ótimo".into(), step: 0, next: Instant::now() + Duration::from_millis(1500), opened: false, typed: false }
    }

    pub fn step(&mut self, app: &mut App, window: &Window) -> bool {
        let now = Instant::now();
        if now < self.next {
            window.request_redraw();
            return false;
        }
        self.next = now + Duration::from_millis(30);
        if !self.opened {
            self.opened = true;
            match &self.quit_note {
                Some(n) => app.open_note(n),
                None => app.show_note("probe.md".into(), String::new(), "h".into()),
            }
            app.go(crate::shell::Page::Notes);
            self.next = now + Duration::from_millis(600);
            return false;
        }
        if self.step < self.script.len() {
            let (k, s) = self.script[self.step];
            unsafe {
                press(k, s, true);
                press(k, s, false);
            }
            self.step += 1;
            return false;
        }
        if self.quit_note.is_some() {
            // Menos de 350 ms depois da última tecla: o autosave ainda não rodou.
            unsafe {
                let ns: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
                let none: *mut AnyObject = std::ptr::null_mut();
                let _: () = msg_send![ns, terminate: none];
            }
            return false;
        }
        if !self.typed {
            self.typed = true;
            self.next = now + Duration::from_millis(300);
            return false;
        }
        let got = app.notes.editor.as_ref().map(|e| e.buf.text()).unwrap_or_default();
        let ok = got == self.expected;
        eprintln!("ime probe: expected {:?} got {:?} -> {}", self.expected, got, if ok { "PASS" } else { "FAIL" });
        let _ = (Event::Focus(true), Key::Enter, Mods::default());
        true
    }
}

impl Default for Probe {
    fn default() -> Self {
        Self::new()
    }
}

