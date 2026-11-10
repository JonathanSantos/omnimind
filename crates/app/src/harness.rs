//! Roda o app real sem janela: injeta eventos por roteiro, salva telas, mede quadros.

use crate::command::{Cmd, Msg};
use crate::input::{Event, Key, Mods};
use crate::shell::{App, Page};
use crate::{fonts::FONTS, Clipboard};
use gfx::{Gpu, Offscreen, TextEngine};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

pub struct Harness {
    pub gpu: std::sync::Arc<Gpu>,
    pub app: App,
    pub target: Offscreen,
    pub w: f32,
    pub h: f32,
    pub scale: f32,
    pub frames: u64,
}

impl Harness {
    pub fn new(w: u32, h: u32, scale: f32, tx: Sender<Cmd>, rx: Receiver<Msg>) -> Self {
        let gpu = Gpu::headless();
        let (pw, ph) = ((w as f32 * scale) as u32, (h as f32 * scale) as u32);
        let text = TextEngine::new(FONTS, scale).with_system_fallback();
        let app = App::new(&gpu, pw, ph, scale, text, tx, rx, Clipboard::local());
        let target = Offscreen::new(&gpu, pw, ph);
        Self { gpu, app, target, w: w as f32, h: h as f32, scale, frames: 0 }
    }

    /// (ms de CPU até o submit, ms total, ainda animando)
    pub fn frame(&mut self, dt: f32) -> (f64, f64, bool) {
        let t = Instant::now();
        let mut enc = self.gpu.device.create_command_encoder(&Default::default());
        let out = self.app.frame(&self.gpu, &mut enc, &self.target.view, self.w, self.h, self.scale, dt);
        let sub = self.gpu.queue.submit([enc.finish()]);
        let cpu = t.elapsed().as_secs_f64() * 1e3;
        self.gpu.device.poll(gfx::wgpu::PollType::Wait { submission_index: Some(sub), timeout: None }).expect("poll");
        self.frames += 1;
        (cpu, t.elapsed().as_secs_f64() * 1e3, out.animating)
    }

    pub fn run(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(1.0 / 60.0);
        }
    }

    /// Avança até nada mais se mover, ou até o limite. Devolve os quadros gastos.
    pub fn settle(&mut self, limit: usize) -> usize {
        for i in 0..limit {
            if !self.frame(1.0 / 60.0).2 && i > 2 {
                return i;
            }
        }
        limit
    }

    pub fn shot(&self, path: &str) {
        self.target.save_png(&self.gpu, Path::new(path)).expect("png");
    }

    pub fn key(&mut self, k: Key, m: Mods) {
        self.app.event(Event::Key(k, m));
    }

    pub fn cmd(&mut self, c: char) {
        self.key(Key::Char(c), Mods { cmd: true, ..Default::default() });
    }

    pub fn typ(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '\n' => self.key(Key::Enter, Mods::default()),
                c => self.app.event(Event::Text(c.to_string())),
            }
        }
    }

    pub fn click(&mut self, x: f32, y: f32) {
        self.app.event(Event::MouseMove(x, y));
        self.frame(1.0 / 60.0);
        self.app.event(Event::MouseDown(x, y, 1));
        self.frame(1.0 / 60.0);
        self.app.event(Event::MouseUp(x, y));
        self.frame(1.0 / 60.0);
    }

    pub fn page(&mut self, p: Page) {
        self.app.go(p);
    }
}

pub fn percentiles(v: &mut [f64]) -> (f64, f64, f64) {
    v.sort_by(|a, b| a.total_cmp(b));
    let at = |p: f64| v[((v.len() - 1) as f64 * p) as usize];
    (at(0.5), at(0.99), v[v.len() - 1])
}
