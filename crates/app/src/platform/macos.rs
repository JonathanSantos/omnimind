//! Pontes com o macOS, via Objective-C: área de transferência, seletor de pasta, saída segura.

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Bool, Imp, Sel};
use objc2::{class, msg_send, msg_send_id, sel};
use std::ffi::CString;
use std::sync::atomic::{AtomicPtr, Ordering};

unsafe fn ns_string(s: &str) -> Retained<AnyObject> {
    let c = CString::new(s.replace('\0', "")).unwrap_or_default();
    msg_send_id![class!(NSString), stringWithUTF8String: c.as_ptr()]
}

unsafe fn to_rust(s: *mut AnyObject) -> String {
    if s.is_null() {
        return String::new();
    }
    let p: *const std::os::raw::c_char = msg_send![s, UTF8String];
    if p.is_null() { String::new() } else { std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned() }
}

pub fn clipboard_set(t: &str) {
    unsafe {
        let pb: *mut AnyObject = msg_send![class!(NSPasteboard), generalPasteboard];
        let _: isize = msg_send![pb, clearContents];
        let kind = ns_string("public.utf8-plain-text");
        let text = ns_string(t);
        let _: Bool = msg_send![pb, setString: &*text, forType: &*kind];
    }
}

pub fn clipboard_get() -> String {
    unsafe {
        let pb: *mut AnyObject = msg_send![class!(NSPasteboard), generalPasteboard];
        let kind = ns_string("public.utf8-plain-text");
        let s: *mut AnyObject = msg_send![pb, stringForType: &*kind];
        to_rust(s)
    }
}

pub fn pick_folder(prompt: &str) -> Option<String> {
    unsafe {
        let panel: *mut AnyObject = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![panel, setCanChooseDirectories: true];
        let _: () = msg_send![panel, setCanChooseFiles: false];
        let _: () = msg_send![panel, setCanCreateDirectories: true];
        let _: () = msg_send![panel, setAllowsMultipleSelection: false];
        let message = ns_string(prompt);
        let _: () = msg_send![panel, setMessage: &*message];
        let answer: isize = msg_send![panel, runModal];
        if answer != 1 {
            return None;
        }
        let url: *mut AnyObject = msg_send![panel, URL];
        if url.is_null() {
            return None;
        }
        let path: *mut AnyObject = msg_send![url, path];
        Some(to_rust(path))
    }
}

/// Preferência de acessibilidade do sistema: sem movimento, o grafo aparece pronto.
pub fn reduce_motion() -> bool {
    unsafe {
        let ws: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
        if ws.is_null() {
            return false;
        }
        let on: Bool = msg_send![ws, accessibilityDisplayShouldReduceMotion];
        on.as_bool()
    }
}

/// Segundos entre o relógio local e o UTC, agora.
pub fn utc_offset() -> i64 {
    unsafe {
        let tz: *mut AnyObject = msg_send![class!(NSTimeZone), localTimeZone];
        if tz.is_null() {
            return 0;
        }
        let s: isize = msg_send![tz, secondsFromGMT];
        s as i64
    }
}

static ON_QUIT: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

extern "C" fn should_terminate(_this: *mut AnyObject, _cmd: Sel, _app: *mut AnyObject) -> usize {
    let p = ON_QUIT.load(Ordering::SeqCst);
    if !p.is_null() {
        let f: fn() = unsafe { std::mem::transmute(p) };
        f();
    }
    // NSTerminateCancel: quem sai é o nosso laço, depois de gravar tudo.
    0
}

/// Cmd+Q, Dock e logout passam por `terminate:` e nunca chegam ao winit como evento.
/// Pendura `applicationShouldTerminate:` no delegate para tudo cair no mesmo caminho.
pub fn hook_terminate(on_quit: fn()) {
    ON_QUIT.store(on_quit as *mut (), Ordering::SeqCst);
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        if delegate.is_null() {
            return;
        }
        let cls: *const AnyClass = objc2::ffi::object_getClass(delegate.cast()).cast();
        let imp: Imp = std::mem::transmute(should_terminate as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> usize);
        let types = CString::new("Q@:@").unwrap();
        objc2::ffi::class_addMethod(cls as *mut _, std::mem::transmute(sel!(applicationShouldTerminate:)), Some(std::mem::transmute(imp)), types.as_ptr());
    }
}
