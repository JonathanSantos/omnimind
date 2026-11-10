//! Universo: a tela em três dimensões. Guarda a câmera, anima abertura, leitura e nascimento,
//! e responde ao mouse. A posição de cada corpo vem pronta de `graph::cosmos`.

use super::gpu::Target;
use super::universe_gpu::{BodyI, Call, DrawU, FrameU, LinkI, Pipes};
use super::{ease_out, smooth};
use crate::theme::{is_dark, ui as ui_style, THEME};
use crate::ui::Ui;
use gfx::wgpu::CommandEncoder;
use gfx::{Gpu, Rect};
use graph::cosmos::{self, arc, lerp, mul, Basis, Camera, Cosmos, FOCAL, V3};
use graph::rng;
use std::sync::Arc;
use std::time::Instant;

/// Tempo que a luz leva de um corpo ao seguinte.
const TRAVEL: f32 = 0.95;
const READ_LIFE: f32 = 5.0;
/// Duas leituras dentro desse intervalo fazem parte do mesmo percurso.
const STREAK: f32 = 25.0;
const MAX_READS: usize = 16;
const BIRTH_SECONDS: f32 = 1.8;
const ORBIT: f32 = 0.006;
const NEUTRAL: f32 = 7.0;

struct Read {
    item: u32,
    from: Option<u32>,
    start: f32,
}

struct Birth {
    item: u32,
    start: f32,
}

struct Drag {
    last: (f32, f32),
    travelled: f32,
    moved_at: Instant,
}

pub struct Universe {
    cosmos: Option<Arc<Cosmos>>,
    pipes: Option<Pipes>,
    uploaded: bool,
    base_bodies: u32,
    base_links: u32,
    pub cam: Camera,
    framed_for: f32,
    spin: (f32, f32),
    drag: Option<Drag>,
    clock: f32,
    open_at: Option<f32>,
    reads: Vec<Read>,
    last_read: Option<(u32, f32)>,
    births: Vec<Birth>,
    focus_item: Option<u32>,
    focus_shown: Option<u32>,
    focus_amt: f32,
    reading: f32,
    pub hover: Option<u32>,
    last_mouse: (f32, f32),
    picked_with: Camera,
    labels_alpha: f32,
    dirty: bool,
}

impl Universe {
    pub fn new() -> Self {
        let cam = Camera { yaw: 0.55, pitch: 0.32, dist: 900.0 };
        Self {
            cosmos: None,
            pipes: None,
            uploaded: false,
            base_bodies: 0,
            base_links: 0,
            cam,
            framed_for: 0.0,
            spin: (0.0, 0.0),
            drag: None,
            clock: 0.0,
            open_at: None,
            reads: vec![],
            last_read: None,
            births: vec![],
            focus_item: None,
            focus_shown: None,
            focus_amt: 0.0,
            reading: 0.0,
            hover: None,
            last_mouse: (-1.0, -1.0),
            picked_with: cam,
            labels_alpha: 0.0,
            dirty: true,
        }
    }

    pub fn active(&self) -> bool {
        self.cosmos.is_some()
    }

    pub fn clear(&mut self) {
        self.cosmos = None;
        self.reads.clear();
        self.births.clear();
        self.hover = None;
    }

    /// Geometria nova. `open` refaz a abertura; `fresh` são os corpos que nascem agora.
    pub fn install(&mut self, c: Cosmos, open: bool, fresh: &[u32]) {
        self.births = fresh.iter().enumerate().map(|(k, &item)| Birth { item, start: self.clock + 0.3 + k as f32 * 0.9 }).collect();
        self.reads.clear();
        self.last_read = None;
        self.hover = None;
        self.focus_item = None;
        self.focus_shown = None;
        self.focus_amt = 0.0;
        self.drag = None;
        if open {
            self.open_at = Some(self.clock);
            self.labels_alpha = 0.0;
        }
        self.cosmos = Some(Arc::new(c));
        self.uploaded = false;
        self.dirty = true;
    }

    /// Todos os corpos novos já viraram parte do céu: hora de refazer a base com eles.
    pub fn births_done(&self) -> bool {
        !self.births.is_empty() && self.births.iter().all(|b| self.clock >= b.start + BIRTH_SECONDS)
    }

    /// A IA leu este corpo: a luz sai do último lido e chega nele.
    pub fn live_read(&mut self, item: u32) {
        let Some(c) = &self.cosmos else { return };
        if item as usize >= c.bodies.len() {
            return;
        }
        self.reads.retain(|r| self.clock - r.start < READ_LIFE);
        if self.reads.len() >= MAX_READS {
            self.reads.remove(0);
        }
        let from = self.last_read.filter(|(i, at)| *i != item && self.clock - at < STREAK).map(|(i, _)| i);
        self.reads.push(Read { item, from, start: self.clock });
        self.last_read = Some((item, self.clock));
        self.dirty = true;
    }

    pub fn reads_shown(&self) -> usize {
        self.reads.iter().filter(|r| self.clock - r.start < READ_LIFE).count()
    }

    fn opening(&self, c: &Cosmos) -> Option<f32> {
        self.open_at.map(|at| self.clock - at).filter(|age| *age < c.open_seconds)
    }

    /// Quanto o céu já se abriu, de 0 a 1: na chegada os corpos saem do centro.
    fn spread(&self, c: &Cosmos) -> f32 {
        self.opening(c).map_or(1.0, |age| 0.08 + 0.92 * ease_out((age / (c.open_seconds * 0.55)).min(1.0)))
    }

    fn hidden(&self, item: u32) -> bool {
        self.births.iter().any(|b| b.item == item && b.start > self.clock)
    }

    pub fn screen_of(&self, item: u32, r: Rect) -> Option<(f32, f32)> {
        let c = self.cosmos.as_ref()?;
        let b = c.bodies.get(item as usize)?;
        let s = self.cam.basis().see(mul(b.pos, self.spread(c)), r.h)?;
        Some((r.x + r.w * 0.5 + s.x, r.y + r.h * 0.5 + s.y))
    }

    fn pick(&self, c: &Cosmos, basis: &Basis, r: Rect, mx: f32, my: f32) -> Option<u32> {
        let (cx, cy) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
        let spread = self.spread(c);
        let mut best = (f32::MAX, None);
        for (i, b) in c.bodies.iter().enumerate() {
            let Some(s) = basis.see(mul(b.pos, spread), r.h) else { continue };
            let d = (cx + s.x - mx).hypot(cy + s.y - my);
            let reach = (b.size * s.scale * 0.6).max(9.0);
            // Entre dois sob o mouse, ganha o mais perto do ponteiro e, no empate, o da frente.
            let score = d + s.depth * 0.002;
            if d <= reach && score < best.0 && !self.hidden(i as u32) {
                best = (score, Some(i as u32));
            }
        }
        best.1
    }

    fn set_focus(&mut self, item: Option<u32>) {
        if item == self.focus_item {
            return;
        }
        self.focus_item = item;
        if item.is_some() {
            self.focus_shown = item;
        }
        self.dirty = true;
    }

    /// Avança a animação, trata o mouse e escreve os nomes. Devolve o clique:
    /// `Some(Some(item))` num corpo, `Some(None)` no vazio.
    pub fn paint(&mut self, ui: &mut Ui<'_>, r: Rect, selected: Option<u32>, interactive: bool) -> Option<Option<u32>> {
        let c = self.cosmos.clone()?;
        if (self.framed_for - c.radius).abs() > 0.5 {
            // Na primeira vez enquadra tudo; depois preserva a aproximação que a pessoa escolheu.
            let fit = Camera::fit(c.radius, r.w, r.h);
            self.cam.dist = if self.framed_for > 0.0 { self.cam.dist * c.radius / self.framed_for } else { fit };
            self.framed_for = c.radius;
            self.dirty = true;
        }
        let dt = ui.dt;
        let opening = self.opening(&c);
        let busy = opening.is_some() || !self.births.is_empty() || self.reads.iter().any(|x| self.clock - x.start < READ_LIFE);
        if busy {
            self.clock += dt;
            self.dirty = true;
            ui.animating = true;
        }
        // O céu gira um pouco enquanto se abre e enquanto a IA lê. Parado, fica parado.
        if let Some(age) = opening {
            self.cam.yaw += dt * 0.22 * (1.0 - age / c.open_seconds);
        }
        if self.reading > 0.01 && self.drag.is_none() {
            self.cam.yaw += dt * 0.045 * self.reading;
        }
        let mut clicked = None;
        let stage = crate::ui::id("graph-stage");
        if interactive {
            let (mx, my) = ui.st.mouse;
            let inside = ui.hover(r);
            if inside && ui.st.clicked {
                ui.st.active = Some(stage);
                self.drag = Some(Drag { last: (mx, my), travelled: 0.0, moved_at: Instant::now() });
                self.spin = (0.0, 0.0);
            }
            if let Some(d) = &mut self.drag {
                let (dx, dy) = (mx - d.last.0, my - d.last.1);
                if ui.st.down && (dx != 0.0 || dy != 0.0) {
                    let secs = d.moved_at.elapsed().as_secs_f32().clamp(1.0 / 240.0, 0.1);
                    d.last = (mx, my);
                    d.travelled += dx.abs() + dy.abs();
                    d.moved_at = Instant::now();
                    self.cam.yaw -= dx * ORBIT;
                    self.cam.pitch = (self.cam.pitch + dy * ORBIT).clamp(-1.45, 1.45);
                    // Impulso para depois de soltar, com teto: um tranco no mouse não vira pião.
                    let cap = |v: f32| v.clamp(-4.0, 4.0);
                    self.spin = (cap(self.spin.0 * 0.5 - dx * ORBIT / secs * 0.5), cap(self.spin.1 * 0.5 + dy * ORBIT / secs * 0.5));
                    self.dirty = true;
                }
            }
            if ui.st.released && ui.st.active == Some(stage) {
                ui.st.active = None;
                if let Some(d) = self.drag.take() {
                    if d.travelled < 4.0 {
                        self.spin = (0.0, 0.0);
                        if inside {
                            clicked = Some(self.hover);
                        }
                    } else if d.moved_at.elapsed().as_millis() > 70 {
                        // Parou antes de soltar: não há impulso para continuar.
                        self.spin = (0.0, 0.0);
                    }
                }
            }
            let zoom = if inside { ui.st.wheel.1 * 0.0022 + ui.st.pinch * 1.4 } else { 0.0 };
            if zoom != 0.0 {
                self.cam.dist = (self.cam.dist * (-zoom).exp()).clamp(c.radius * 0.45, c.radius * 6.0);
                self.dirty = true;
            }
            let turning = self.drag.as_ref().is_some_and(|d| d.travelled >= 4.0);
            if !inside || turning {
                self.hover = None;
            } else if (mx, my) != self.last_mouse || self.picked_with != self.cam || ui.st.clicked {
                self.last_mouse = (mx, my);
                self.picked_with = self.cam;
                self.hover = self.pick(&c, &self.cam.basis(), r, mx, my);
            }
            self.set_focus(self.hover.or(selected));
        }
        if self.drag.is_none() && (self.spin.0.abs() > 0.02 || self.spin.1.abs() > 0.02) {
            self.cam.yaw += self.spin.0 * dt;
            self.cam.pitch = (self.cam.pitch + self.spin.1 * dt).clamp(-1.45, 1.45);
            let k = (-dt * 3.2).exp();
            self.spin = (self.spin.0 * k, self.spin.1 * k);
            self.dirty = true;
            ui.animating = true;
        }
        let goal = self.focus_item.is_some() as u32 as f32;
        if (self.focus_amt - goal).abs() > 0.001 {
            let step = dt * 7.0;
            self.focus_amt = if goal > self.focus_amt { (self.focus_amt + step).min(1.0) } else { (self.focus_amt - step).max(0.0) };
            self.dirty = true;
            ui.animating = true;
        }
        let reading_goal = self.reads.iter().any(|x| self.clock - x.start < READ_LIFE - 1.6) as u32 as f32;
        if (self.reading - reading_goal).abs() > 0.002 {
            self.reading += (reading_goal - self.reading) * (1.0 - (-dt * 3.0).exp());
            self.dirty = true;
            ui.animating = true;
        } else {
            self.reading = reading_goal;
        }
        let goal = opening.is_none_or(|age| age > c.open_seconds * 0.6) as u32 as f32;
        if (self.labels_alpha - goal).abs() > 0.01 {
            self.labels_alpha += (goal - self.labels_alpha) * (1.0 - (-dt * 6.0).exp());
            ui.animating = true;
        } else {
            self.labels_alpha = goal;
        }
        self.labels(ui, &c, r);
        clicked
    }

    /// Nome de cada tema, acima do seu aglomerado. Os de trás ficam mais apagados.
    fn labels(&self, ui: &mut Ui<'_>, c: &Cosmos, r: Rect) {
        let basis = self.cam.basis();
        let spread = self.spread(c);
        let (cx, cy) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
        let (near, far) = (self.cam.dist - c.radius, self.cam.dist + c.radius);
        ui.s.push_clip(r);
        for cl in c.clusters.iter().filter(|cl| !cl.name.is_empty()) {
            let Some(s) = basis.see(mul(cl.center, spread), r.h) else { continue };
            let depth = ((s.depth - near) / (far - near).max(1.0)).clamp(0.0, 1.0);
            let a = self.labels_alpha * (1.0 - 0.5 * depth) * (1.0 - 0.45 * smooth(self.focus_amt));
            let lay = ui.line(&cl.name, ui_style(12.0).weight(500).color(THEME.sub.alpha(a)));
            let (x, y) = (cx + s.x - lay.width * 0.5 + 7.0, cy + s.y - cl.radius * spread * s.scale - lay.height - 8.0);
            for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                ui.s.text_tinted(ui.t, &lay, x + dx, y + dy, Some(THEME.bg), 0.9 * a);
            }
            ui.draw(&lay, x, y);
            // Um aglomerado na frente de outro confunde de quem é o nome: o ponto leva a cor do tema.
            let dot = THEME.themes[(cl.theme as usize).min(THEME.themes.len() - 1)];
            ui.s.round(Rect::new(x - 13.0, y + lay.height * 0.5 - 3.5, 7.0, 7.0), 3.5, dot.alpha(a));
        }
        // Projetos: um nome menor em cada sistema. Com muitos, só de perto cabem sem se atropelar.
        let close = self.framed_for > 0.0 && self.cam.dist < Camera::fit(self.framed_for, r.w, r.h) * 0.75;
        if c.systems.len() <= 24 || close {
            for sys in &c.systems {
                let Some(s) = basis.see(mul(sys.center, spread), r.h) else { continue };
                let depth = ((s.depth - near) / (far - near).max(1.0)).clamp(0.0, 1.0);
                let a = self.labels_alpha * (0.85 - 0.5 * depth) * (1.0 - 0.45 * smooth(self.focus_amt));
                let lay = ui.line(&sys.name, ui_style(11.0).color(THEME.mute.alpha(a)));
                let (x, y) = (cx + s.x - lay.width * 0.5, cy + s.y + sys.radius * spread * s.scale + 4.0);
                for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                    ui.s.text_tinted(ui.t, &lay, x + dx, y + dy, Some(THEME.bg), 0.9 * a);
                }
                ui.draw(&lay, x, y);
            }
        }
        ui.s.pop_clip();
    }

    /// O que nunca muda entre quadros: poeira de fundo, névoa dos temas, corpos e linhas.
    fn upload(&mut self, gpu: &Gpu, c: &Cosmos) {
        let pipes = self.pipes.get_or_insert_with(|| Pipes::new(gpu));
        let mut r = rng(0xC05A_05D0);
        let mut bodies: Vec<BodyI> = Vec::with_capacity(c.bodies.len() + 512);
        let dust = if c.tier == 2 { 280 } else { 420 };
        for _ in 0..dust {
            let dir = cosmos::unit([r() * 2.0 - 1.0, r() * 2.0 - 1.0, r() * 2.0 - 1.0]);
            bodies.push(BodyI { pos: mul(dir, c.radius * (3.4 + r() * 2.0)), size: 1.3 + r() * r() * 2.4, theme: NEUTRAL, alpha: 0.25 + r() * 0.6, delay: 0.0, kind: 3.0 });
        }
        for cl in &c.clusters {
            bodies.push(BodyI { pos: cl.center, size: cl.radius * 1.8, theme: cl.theme as f32, alpha: 0.2, delay: 0.04, kind: 2.0 });
        }
        for b in c.bodies.iter().filter(|b| !b.fresh) {
            bodies.push(BodyI { pos: b.pos, size: b.size, theme: b.theme as f32, alpha: 1.0, delay: b.delay, kind: b.note as u32 as f32 });
        }
        let alpha = [0.38, 0.28, 0.17][c.tier];
        let links: Vec<LinkI> = c
            .lines
            .iter()
            .filter(|(a, b)| !c.bodies[*a as usize].fresh && !c.bodies[*b as usize].fresh)
            .map(|&(a, b)| {
                let (x, y) = (&c.bodies[a as usize], &c.bodies[b as usize]);
                LinkI { a: x.pos, theme_a: x.theme as f32, b: y.pos, theme_b: y.theme as f32, alpha, width: 1.7, delay: x.delay.max(y.delay) + 0.08 + r() * 0.2, pad: 0.0 }
            })
            .collect();
        pipes.bodies.write(gpu, bytemuck::cast_slice(&bodies));
        pipes.links.write(gpu, bytemuck::cast_slice(&links));
        self.base_bodies = bodies.len() as u32;
        self.base_links = links.len() as u32;
        self.uploaded = true;
    }

    /// Tudo o que está aceso agora: foco do mouse, percurso da leitura e corpos nascendo.
    fn effects(&self, c: &Cosmos) -> (Vec<BodyI>, Vec<LinkI>) {
        let (mut bodies, mut links) = (Vec::new(), Vec::new());
        let t = self.clock;
        let glow = |v: &mut Vec<BodyI>, pos: V3, size: f32, theme: f32, alpha: f32| v.push(BodyI { pos, size, theme, alpha, delay: 0.0, kind: 2.0 });
        let body = |v: &mut Vec<BodyI>, i: u32, grow: f32, alpha: f32| {
            let b = &c.bodies[i as usize];
            v.push(BodyI { pos: b.pos, size: b.size * grow, theme: b.theme as f32, alpha, delay: 0.0, kind: b.note as u32 as f32 });
        };
        let link = |v: &mut Vec<LinkI>, a: V3, ta: u8, b: V3, tb: u8, alpha: f32, width: f32| v.push(LinkI { a, theme_a: ta as f32, b, theme_b: tb as f32, alpha, width, delay: 0.0, pad: 0.0 });

        if let (Some(i), true) = (self.focus_shown, self.focus_amt > 0.0) {
            let a = smooth(self.focus_amt);
            let me = &c.bodies[i as usize];
            for &n in &me.neighbors {
                if self.hidden(n) {
                    continue;
                }
                let other = &c.bodies[n as usize];
                link(&mut links, me.pos, me.theme, other.pos, other.theme, 0.9 * a, 2.3);
                body(&mut bodies, n, 1.0, 0.9 * a);
                glow(&mut bodies, other.pos, other.size * 2.2, other.theme as f32, 0.4 * a);
            }
            body(&mut bodies, i, 1.15, a);
            glow(&mut bodies, me.pos, me.size * 3.4, me.theme as f32, 0.7 * a);
        }

        for r in &self.reads {
            let age = t - r.start;
            if !(0.0..READ_LIFE).contains(&age) {
                continue;
            }
            let fade = ((READ_LIFE - age) / 1.6).clamp(0.0, 1.0);
            let to = &c.bodies[r.item as usize];
            let mut arrived = age;
            if let Some(f) = r.from {
                let from = &c.bodies[f as usize];
                let p = smooth(age / TRAVEL);
                const STEPS: usize = 28;
                for k in 0..STEPS {
                    let (t0, t1) = (k as f32 / STEPS as f32, (k + 1) as f32 / STEPS as f32);
                    if t0 >= p {
                        break;
                    }
                    let end = t1.min(p);
                    // Perto da ponta a linha brilha mais: é o rastro da luz que viaja.
                    let trail = if age < TRAVEL { (1.0 - (p - end) * 2.6).clamp(0.0, 1.0) } else { 0.0 };
                    let (ta, tb) = (if t0 < 0.5 { from.theme } else { to.theme }, if end <= 0.5 { from.theme } else { to.theme });
                    link(&mut links, arc(from.pos, to.pos, t0), ta, arc(from.pos, to.pos, end), tb, (0.5 + 0.5 * trail) * fade, 2.2 + 1.6 * trail);
                }
                if age < TRAVEL {
                    let head = arc(from.pos, to.pos, p);
                    glow(&mut bodies, head, 9.0, NEUTRAL, 1.0);
                    glow(&mut bodies, head, 24.0, to.theme as f32, 0.55);
                }
                arrived = age - TRAVEL;
            }
            if arrived < 0.0 {
                continue;
            }
            let e = (arrived / 1.2).min(1.0);
            glow(&mut bodies, to.pos, to.size * (2.0 + 5.5 * ease_out(e)), to.theme as f32, (1.0 - e) * (1.0 - e) * 0.95);
            body(&mut bodies, r.item, 1.2, fade);
            glow(&mut bodies, to.pos, to.size * 2.8, to.theme as f32, 0.6 * fade);
            // Chegou: as ligações do corpo lido acendem até os vizinhos.
            let q = ease_out((arrived / 0.6).min(1.0));
            for &n in to.neighbors.iter().take(48) {
                if self.hidden(n) {
                    continue;
                }
                let other = &c.bodies[n as usize];
                link(&mut links, to.pos, to.theme, lerp(to.pos, other.pos, q), other.theme, 0.7 * fade, 2.0);
                if q >= 1.0 {
                    glow(&mut bodies, other.pos, other.size * 2.0, other.theme as f32, 0.35 * fade);
                }
            }
        }

        for b in &self.births {
            let age = t - b.start;
            if age < 0.0 {
                continue;
            }
            let me = &c.bodies[b.item as usize];
            body(&mut bodies, b.item, 0.3 + 0.7 * ease_out((age / 0.7).min(1.0)), 1.0);
            let e = (age / 1.3).min(1.0);
            glow(&mut bodies, me.pos, me.size * (2.0 + 6.0 * ease_out(e)), me.theme as f32, (1.0 - e) * (1.0 - e));
            let q = smooth((age - 0.4) / 0.8);
            if q > 0.0 {
                for &n in me.neighbors.iter().take(12) {
                    if self.hidden(n) {
                        continue;
                    }
                    let other = &c.bodies[n as usize];
                    link(&mut links, me.pos, me.theme, lerp(me.pos, other.pos, q), other.theme, 0.5, 1.8);
                }
            }
        }
        bodies.truncate(8000);
        links.truncate(8000);
        (bodies, links)
    }

    /// Desenha na textura só quando algo mudou.
    pub fn render(&mut self, gpu: &Gpu, enc: &mut CommandEncoder, target: &Target, force: bool) {
        let Some(c) = self.cosmos.clone() else { return };
        if !self.dirty && !force && self.uploaded {
            return;
        }
        if !self.uploaded {
            self.upload(gpu, &c);
        }
        self.dirty = false;
        let (fx_bodies, fx_links) = self.effects(&c);
        let opening = self.opening(&c);
        let spread = self.spread(&c);
        let dim = (1.0 - 0.55 * self.reading) * (1.0 - 0.6 * smooth(self.focus_amt));
        let base = DrawU { open: opening.map_or(9.0, |age| age / c.open_seconds * 1.1), spread, dim, pad: 0.0 };
        let lit = DrawU { open: 9.0, spread, dim: 1.0, pad: 0.0 };
        let Some(pipes) = &mut self.pipes else { return };
        pipes.fx_bodies.write(gpu, bytemuck::cast_slice(&fx_bodies));
        pipes.fx_links.write(gpu, bytemuck::cast_slice(&fx_links));
        let b = self.cam.basis();
        let v4 = |v: V3| [v[0], v[1], v[2], 0.0];
        let mut pal = [[0f32; 4]; 8];
        for (i, col) in THEME.themes.iter().enumerate() {
            pal[i] = [col.r, col.g, col.b, 1.0];
        }
        pal[7] = [THEME.ink.r, THEME.ink.g, THEME.ink.b, 1.0];
        let frame = FrameU {
            right: v4(b.right),
            up: v4(b.up),
            fwd: v4(b.fwd),
            eye: v4(b.eye),
            res: [target.size.0 as f32, target.size.1 as f32],
            focal: FOCAL,
            light: !is_dark() as u32 as f32,
            fog: [self.cam.dist - c.radius, self.cam.dist + c.radius],
            pad: [0.0; 2],
            pal,
        };
        let calls = [(Call::Links(self.base_links), base), (Call::Bodies(self.base_bodies), base), (Call::FxLinks(fx_links.len() as u32), lit), (Call::FxBodies(fx_bodies.len() as u32), lit)];
        pipes.draw(gpu, enc, target, frame, [THEME.bg.r, THEME.bg.g, THEME.bg.b], &calls);
    }
}

impl Default for Universe {
    fn default() -> Self {
        Self::new()
    }
}
