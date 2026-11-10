//! Janela nativa. Redesenha sob demanda: parado, o app não gasta um quadro.
// No Windows, o app instalado não abre um console atrás da janela.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use app::command::{Cmd, Msg};
use app::input::{Event, Key, Mods};
use app::shell::App;
use app::{fonts::FONTS, platform, Clipboard};
use gfx::wgpu::{self, CompositeAlphaMode, PresentMode, SurfaceConfiguration, TextureUsages};
use gfx::{Gpu, TextEngine, UI_FORMAT};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key as WKey, NamedKey};
#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::window::{Theme as WTheme, Window, WindowId};

#[derive(Debug, Clone, Copy)]
enum UserEvent {
    Wake,
    Quit,
}

static PROXY: OnceLock<EventLoopProxy<UserEvent>> = OnceLock::new();

fn request_quit() {
    if let Some(p) = PROXY.get() {
        let _ = p.send_event(UserEvent::Quit);
    }
}

struct Live {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: SurfaceConfiguration,
    gpu: Arc<Gpu>,
    app: App,
}

struct Runner {
    live: Option<Live>,
    boot: Option<(Sender<Cmd>, Receiver<Msg>)>,
    data: Option<std::thread::JoinHandle<()>>,
    last: Instant,
    mods: Mods,
    mouse: (f32, f32),
    last_click: (Instant, f32, f32, u32),
    wake_at: Option<Instant>,
    frames: u64,
    measure: Option<Vec<f64>>,
    measure_until: u64,
    measure_len: u64,
    probe: Option<app::probe::Probe>,
    born: Instant,
    idle_check: Option<u64>,
}

impl Runner {
    fn scale(&self) -> f32 {
        self.live.as_ref().map_or(1.0, |l| l.window.scale_factor() as f32)
    }

    fn redraw(&self) {
        if let Some(l) = &self.live {
            l.window.request_redraw();
        }
    }

    fn send(&mut self, e: Event) {
        if let Some(l) = &mut self.live {
            l.app.event(e);
        }
        self.redraw();
    }

    fn quit(&mut self, el: &ActiveEventLoop) {
        if let Some(l) = &mut self.live {
            l.app.flush_all();
        }
        // Quem grava é a thread de dados: esperar por ela é o que garante a última tecla.
        if let Some(h) = self.data.take() {
            let start = Instant::now();
            while !h.is_finished() && start.elapsed() < Duration::from_secs(3) {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        el.exit();
    }

    fn render(&mut self) {
        let dt = self.last.elapsed().as_secs_f32().min(0.05);
        let frame_ms = self.last.elapsed().as_secs_f64() * 1e3;
        self.last = Instant::now();
        let Some(l) = &mut self.live else { return };
        let frame = match l.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => {
                // Superfície perdida ou desatualizada: reconfigura e tenta de novo.
                // Sem pedir outro quadro, a tela ficaria velha até o próximo evento.
                l.surface.configure(&l.gpu.device, &l.config);
                l.window.request_redraw();
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        let scale = l.window.scale_factor() as f32;
        let (lw, lh) = (l.config.width as f32 / scale, l.config.height as f32 / scale);
        let mut enc = l.gpu.device.create_command_encoder(&Default::default());
        let out = l.app.frame(&l.gpu, &mut enc, &view, lw, lh, scale, dt);
        l.gpu.queue.submit([enc.finish()]);
        l.window.pre_present_notify();
        l.gpu.queue.present(frame);
        if out.drag_window {
            let _ = l.window.drag_window();
        }
        if let Some((x, y, h)) = out.ime_area {
            let p = PhysicalPosition::new((x * scale) as i32, (y * scale) as i32);
            l.window.set_ime_cursor_area(p, PhysicalSize::new(2u32, (h * scale) as u32));
        }
        self.frames += 1;
        if self.frames == 1 {
            if let Ok(t) = std::env::var("OMNIMIND_TRACE_START") {
                eprintln!("first frame after {:?} ({t})", self.born.elapsed());
            }
        }
        // Só conta com dados na tela: medir antes de carregar não prova nada.
        if self.measure.is_some() && l.app.model.graph.nodes.is_empty() {
            self.measure_until = self.frames + self.measure_len;
            l.window.request_redraw();
        }
        if let Some(v) = &mut self.measure {
            if self.frames + self.measure_len > self.measure_until + 30 {
                v.push(frame_ms);
            }
            if self.frames >= self.measure_until {
                let mut v = std::mem::take(v);
                let n = v.len();
                let (p50, p99, max) = app::harness::percentiles(&mut v);
                let over = v.iter().filter(|x| **x > 20.0).count();
                eprintln!("window frames {n}: interval p50 {p50:.2} p99 {p99:.2} max {max:.2} ms; over 20ms: {over}");
                request_quit();
            }
        }
        let keep = self.measure.is_some() || self.probe.is_some();
        if out.animating || keep {
            l.window.request_redraw();
        }
        self.wake_at = out.wake_in.map(|d| Instant::now() + d);
    }
}

impl ApplicationHandler<UserEvent> for Runner {
    fn new_events(&mut self, _el: &ActiveEventLoop, cause: StartCause) {
        if let (StartCause::ResumeTimeReached { .. }, Some(_)) = (cause, self.wake_at) {
            self.redraw();
        }
    }

    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.live.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Omnimind")
            .with_inner_size(LogicalSize::new(1440.0, 900.0))
            .with_min_inner_size(LogicalSize::new(860.0, 560.0));
        // No macOS o conteúdo sobe até a barra de título. Nos outros a janela mantém a moldura do sistema.
        #[cfg(target_os = "macos")]
        let attrs = attrs.with_titlebar_transparent(true).with_title_hidden(true).with_fullsize_content_view(true);
        let window = Arc::new(el.create_window(attrs).expect("window"));
        window.set_ime_allowed(true);
        let instance = Gpu::instance();
        let surface = instance.create_surface(window.clone()).expect("surface");
        let gpu = Gpu::new(Some(&surface), instance);
        let size = window.inner_size();
        let scale = window.scale_factor() as f32;
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: UI_FORMAT,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::Fifo,
            // Medido: com 1, de 20 a 35 quadros perdidos em 300; com 2, nenhum.
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            color_space: Default::default(),
        };
        surface.configure(&gpu.device, &config);
        let (tx, rx) = self.boot.take().expect("boot");
        let text = TextEngine::new(FONTS, scale).with_system_fallback();
        let mut app = App::new(&gpu, config.width, config.height, scale, text, tx, rx, Clipboard::system());
        app.pump();
        app.system_theme(window.theme() == Some(WTheme::Dark));
        platform::hook_terminate(request_quit);
        self.live = Some(Live { window, surface, config, gpu, app });
        self.last = Instant::now();
        self.redraw();
    }

    fn user_event(&mut self, el: &ActiveEventLoop, e: UserEvent) {
        match e {
            UserEvent::Wake => self.redraw(),
            UserEvent::Quit => self.quit(el),
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, e: WindowEvent) {
        match e {
            WindowEvent::CloseRequested => self.quit(el),
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::Resized(s) => {
                if let Some(l) = &mut self.live {
                    l.config.width = s.width.max(1);
                    l.config.height = s.height.max(1);
                    l.surface.configure(&l.gpu.device, &l.config);
                    l.app.resize(&l.gpu, l.config.width, l.config.height);
                    l.window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(l) = &mut self.live {
                    l.app.text.set_scale(scale_factor as f32);
                    l.window.request_redraw();
                }
            }
            // Voltou a aparecer: o que chegou enquanto estava coberta precisa ir para a tela.
            WindowEvent::Occluded(false) => self.redraw(),
            WindowEvent::ThemeChanged(t) => {
                if let Some(l) = &mut self.live {
                    l.app.system_theme(t == WTheme::Dark);
                    l.window.request_redraw();
                }
            }
            WindowEvent::Focused(f) => self.send(Event::Focus(f)),
            WindowEvent::ModifiersChanged(m) => {
                let s = m.state();
                self.mods = mods_of(s.shift_key(), s.control_key(), s.alt_key(), s.super_key());
                if let Some(l) = &mut self.live {
                    l.app.st.mods = self.mods;
                }
            }
            WindowEvent::Ime(Ime::Commit(t)) => self.send(Event::Text(t)),
            WindowEvent::Ime(Ime::Preedit(t, _)) => self.send(Event::Preedit(t)),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let mut m = self.mods;
                let key = match &event.logical_key {
                    WKey::Named(NamedKey::Enter) => Some(Key::Enter),
                    WKey::Named(NamedKey::Escape) => Some(Key::Escape),
                    WKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
                    WKey::Named(NamedKey::Delete) => Some(Key::Delete),
                    WKey::Named(NamedKey::Tab) => Some(Key::Tab),
                    WKey::Named(NamedKey::ArrowLeft) => Some(Key::Left),
                    WKey::Named(NamedKey::ArrowRight) => Some(Key::Right),
                    WKey::Named(NamedKey::ArrowUp) => Some(Key::Up),
                    WKey::Named(NamedKey::ArrowDown) => Some(Key::Down),
                    WKey::Named(NamedKey::Home) => Some(Key::Home),
                    WKey::Named(NamedKey::End) => Some(Key::End),
                    WKey::Named(NamedKey::PageUp) => Some(Key::PageUp),
                    WKey::Named(NamedKey::PageDown) => Some(Key::PageDown),
                    WKey::Named(NamedKey::Space) if !m.cmd => None,
                    WKey::Character(c) if m.cmd || m.ctrl => c.chars().next().map(|c| Key::Char(c.to_ascii_lowercase())),
                    _ => None,
                };
                if let Some(k) = key {
                    // Fora do macOS, Ctrl + seta e Ctrl + apagar andam por palavra, como a pessoa espera.
                    if !cfg!(target_os = "macos") && m.cmd && matches!(k, Key::Left | Key::Right | Key::Backspace | Key::Delete) {
                        m = Mods { cmd: false, alt: true, ..m };
                    }
                    self.send(Event::Key(k, m));
                } else if let Some(t) = event.text {
                    let s = t.to_string();
                    if !s.chars().any(char::is_control) {
                        self.send(Event::Text(s));
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let s = self.scale();
                self.mouse = (position.x as f32 / s, position.y as f32 / s);
                self.send(Event::MouseMove(self.mouse.0, self.mouse.1));
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                let (x, y) = self.mouse;
                if state == ElementState::Pressed {
                    let (t, px, py, n) = self.last_click;
                    let near = (x - px).abs() < 4.0 && (y - py).abs() < 4.0;
                    let clicks = if near && t.elapsed() < Duration::from_millis(350) { n + 1 } else { 1 };
                    self.last_click = (Instant::now(), x, y, clicks);
                    self.send(Event::MouseDown(x, y, clicks));
                } else {
                    self.send(Event::MouseUp(x, y));
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let s = self.scale();
                let (x, y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x * 24.0, y * 24.0),
                    MouseScrollDelta::PixelDelta(p) => (p.x as f32 / s, p.y as f32 / s),
                };
                self.send(Event::Scroll(x, y));
            }
            WindowEvent::PinchGesture { delta, .. } => self.send(Event::Pinch(delta as f32)),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        // --idle: conta os quadros numa janela de 4 s, depois de tudo assentar.
        if std::env::args().any(|a| a == "--idle") {
            let t = self.born.elapsed().as_secs_f32();
            if t > 14.0 && self.idle_check.is_none() {
                self.idle_check = Some(self.frames);
            }
            if t > 18.0 {
                eprintln!("idle frames in 4 s: {}", self.frames - self.idle_check.unwrap_or(self.frames));
                request_quit();
            }
            el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(250)));
            return;
        }
        if let (Some(p), Some(l)) = (&mut self.probe, &mut self.live) {
            if p.step(&mut l.app, &l.window) {
                self.probe = None;
                request_quit();
            }
        }
        el.set_control_flow(match self.wake_at {
            Some(t) => ControlFlow::WaitUntil(t),
            None => ControlFlow::Wait,
        });
    }
}

/// A tecla de comando do app: ⌘ no macOS, Ctrl no Windows e no Linux.
fn mods_of(shift: bool, ctrl: bool, alt: bool, logo: bool) -> Mods {
    if cfg!(target_os = "macos") {
        Mods { shift, ctrl, alt, cmd: logo }
    } else {
        Mods { shift, ctrl: false, alt, cmd: ctrl }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let el = EventLoop::<UserEvent>::with_user_event().build().expect("event loop");
    let proxy = el.create_proxy();
    let _ = PROXY.set(proxy.clone());
    let wake = move || {
        let _ = proxy.send_event(UserEvent::Wake);
    };
    let (ctx, crx) = channel();
    let (mtx, mrx) = channel();
    let brain = args.iter().position(|a| a == "--brain").and_then(|i| args.get(i + 1)).cloned();
    let demo = args.iter().any(|a| a == "--demo");
    let data = app::data::spawn(crx, mtx, Box::new(wake), brain, demo);
    let measure = args.iter().position(|a| a == "--measure").and_then(|i| args.get(i + 1)).and_then(|s| s.parse::<u64>().ok());
    let quit_note = args.iter().position(|a| a == "--probe-quit").and_then(|i| args.get(i + 1)).cloned();
    let probe = match quit_note {
        Some(n) => Some(app::probe::Probe::for_quit(n)),
        None => args.iter().any(|a| a == "--probe-ime").then(app::probe::Probe::new),
    };
    let mut r = Runner {
        live: None,
        boot: Some((ctx, mrx)),
        data: Some(data),
        last: Instant::now(),
        mods: Mods::default(),
        mouse: (0.0, 0.0),
        last_click: (Instant::now(), 0.0, 0.0, 0),
        wake_at: None,
        frames: 0,
        measure: measure.map(|_| Vec::new()),
        measure_until: measure.unwrap_or(0) + 30,
        measure_len: measure.unwrap_or(0) + 30,
        probe,
        born: Instant::now(),
        idle_check: None,
    };
    el.run_app(&mut r).expect("run");
}
