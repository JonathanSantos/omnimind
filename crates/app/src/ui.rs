//! Camada imediata: cada quadro redesenha a partir do estado, e as interações
//! saem do retângulo que acabou de ser pintado. Animação é mola por id.

use crate::input::Mods;
use crate::theme::{ui as ui_style, THEME};
use gfx::{Align, Color, Layout, Rect, Scene, Span, TextEngine, TextStyle};
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub type Id = u64;

pub fn id(s: impl Hash) -> Id {
    let mut h = FxHasher::default();
    s.hash(&mut h);
    h.finish()
}

#[derive(Default)]
pub struct UiState {
    pub mouse: (f32, f32),
    pub down: bool,
    pub clicked: bool,
    pub clicks: u32,
    pub released: bool,
    pub wheel: (f32, f32),
    pub pinch: f32,
    pub mods: Mods,
    pub active: Option<Id>,
    pub hot: Option<Id>,
    anim: FxHashMap<Id, f32>,
    scrolls: FxHashMap<Id, f32>,
    overlays: Vec<Rect>,
    next_overlays: Vec<Rect>,
}

impl UiState {
    pub fn end_frame(&mut self) {
        self.clicked = false;
        self.released = false;
        self.clicks = 0;
        self.wheel = (0.0, 0.0);
        self.pinch = 0.0;
        self.overlays = std::mem::take(&mut self.next_overlays);
    }
}

pub struct Ui<'a> {
    pub s: &'a mut Scene,
    pub t: &'a mut TextEngine,
    pub st: &'a mut UiState,
    pub time: f32,
    pub dt: f32,
    pub size: (f32, f32),
    /// Alguma mola ainda em movimento: o quadro seguinte é necessário.
    pub animating: bool,
    pub in_overlay: bool,
}

impl Ui<'_> {
    /// Valor que persegue `target` com mola criticamente amortecida.
    pub fn anim(&mut self, key: Id, target: f32, rate: f32) -> f32 {
        let v = self.st.anim.entry(key).or_insert(target);
        let k = 1.0 - (-self.dt * rate).exp();
        *v += (target - *v) * k;
        if (target - *v).abs() < 0.002 {
            *v = target;
        } else {
            self.animating = true;
        }
        *v
    }

    pub fn anim_from(&mut self, key: Id, start: f32, target: f32, rate: f32) -> f32 {
        self.st.anim.entry(key).or_insert(start);
        self.anim(key, target, rate)
    }

    pub fn reset_anim(&mut self, key: Id, v: f32) {
        self.st.anim.insert(key, v);
    }

    /// Registra um painel flutuante: o que está por baixo deixa de receber o mouse.
    pub fn overlay(&mut self, r: Rect) {
        self.st.next_overlays.push(r);
    }

    pub fn covered(&self) -> bool {
        let (x, y) = self.st.mouse;
        !self.in_overlay && self.st.overlays.iter().any(|o| o.contains(x, y))
    }

    pub fn hover(&self, r: Rect) -> bool {
        let (x, y) = self.st.mouse;
        r.contains(x, y) && self.s.clip().contains(x, y) && !self.covered()
    }

    /// Clique completo: aperta e solta sobre o mesmo alvo.
    pub fn pressed(&mut self, key: Id, r: Rect) -> bool {
        let over = self.hover(r);
        if over {
            self.st.hot = Some(key);
        }
        if over && self.st.clicked {
            self.st.active = Some(key);
        }
        let fired = self.st.released && self.st.active == Some(key) && over;
        if self.st.released && self.st.active == Some(key) {
            self.st.active = None;
        }
        fired
    }

    pub fn line(&mut self, text: &str, style: TextStyle) -> Arc<Layout> {
        self.t.line(text, style)
    }

    pub fn draw(&mut self, l: &Layout, x: f32, y: f32) {
        self.s.text(self.t, l, x, y);
    }

    /// Texto numa linha com ponto de ancoragem à esquerda; devolve a largura.
    pub fn text(&mut self, text: &str, style: TextStyle, x: f32, y: f32) -> f32 {
        let l = self.t.line(text, style);
        self.s.text(self.t, &l, x, y);
        l.width
    }

    pub fn text_right(&mut self, text: &str, style: TextStyle, right: f32, y: f32) -> f32 {
        let l = self.t.line(text, style);
        self.s.text(self.t, &l, right - l.width, y);
        l.width
    }

    pub fn text_center(&mut self, text: &str, style: TextStyle, cx: f32, y: f32) {
        let l = self.t.line(text, style);
        self.s.text(self.t, &l, cx - l.width * 0.5, y);
    }

    /// Parágrafo com quebra; devolve a altura ocupada.
    pub fn para(&mut self, text: &str, style: TextStyle, x: f32, y: f32, width: f32) -> f32 {
        let l = self.t.layout(&[Span::new(text, style)], Some(width), Align::Left);
        self.s.text(self.t, &l, x, y);
        l.height
    }

    pub fn para_height(&mut self, text: &str, style: TextStyle, width: f32) -> f32 {
        self.t.layout(&[Span::new(text, style)], Some(width), Align::Left).height
    }

    /// Corta com reticências até caber. Medir é barato: o layout fica em cache.
    pub fn ellipsize(&mut self, text: &str, style: TextStyle, width: f32) -> String {
        if self.t.line(text, style).width <= width {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        let (mut lo, mut hi) = (0, chars.len());
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            let s: String = chars[..mid].iter().collect::<String>() + "…";
            if self.t.line(&s, style).width <= width {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        chars[..lo].iter().collect::<String>().trim_end().to_string() + "…"
    }

    /// Botão com contorno. O rótulo fica centrado e o fundo responde ao mouse.
    pub fn button(&mut self, key: Id, r: Rect, label: &str) -> bool {
        let fired = self.pressed(key, r);
        let hover = self.hover(r);
        let a = self.anim(key ^ 1, hover as u32 as f32, 18.0);
        let fill = THEME.hover.alpha(a);
        self.s.panel(r, 8.0, fill, THEME.line.mix(THEME.mute, a), 1.0);
        let l = self.t.line(label, ui_style(13.5).weight(500));
        self.s.text(self.t, &l, r.x + (r.w - l.width) * 0.5, r.y + (r.h - l.height) * 0.5);
        fired
    }

    /// Botão principal: tinta cheia.
    /// Interruptor: o trilho diz o estado, e a posição confirma, para não depender só de cor.
    pub fn toggle(&mut self, key: Id, r: Rect, on: bool) -> bool {
        let hover = self.hover(r);
        let a = self.anim(key ^ 3, on as u32 as f32, 24.0);
        let track = if on { THEME.acc } else if hover { THEME.line.mix(THEME.mute, 0.5) } else { THEME.line };
        self.s.round(r, r.h * 0.5, track);
        let knob = r.h - 6.0;
        let x = r.x + 3.0 + (r.w - knob - 6.0) * a;
        self.s.round(Rect::new(x, r.y + 3.0, knob, knob), knob * 0.5, if on { THEME.bg } else { THEME.ink });
        self.pressed(key, r)
    }

    pub fn primary(&mut self, key: Id, r: Rect, label: &str) -> bool {
        let fired = self.pressed(key, r);
        let hover = self.hover(r);
        let a = self.anim(key ^ 1, hover as u32 as f32, 18.0);
        self.s.round(r, 8.0, THEME.ink.alpha(1.0 - 0.12 * a));
        let l = self.t.line(label, ui_style(14.0).weight(600).color(THEME.bg));
        self.s.text(self.t, &l, r.x + (r.w - l.width) * 0.5, r.y + (r.h - l.height) * 0.5);
        fired
    }

    /// Filtro curto. Devolve (clicado, largura ocupada).
    pub fn chip(&mut self, key: Id, x: f32, y: f32, label: &str, on: bool) -> (bool, f32) {
        let st = ui_style(13.0).weight(if on { 600 } else { 400 }).color(if on { THEME.ink } else { THEME.sub });
        let reserved = self.t.line(label, ui_style(13.0).weight(600)).width;
        let r = Rect::new(x, y, reserved + 20.0, 28.0);
        let fired = self.pressed(key, r);
        let hover = self.hover(r);
        let a = self.anim(key ^ 1, on as u32 as f32, 18.0);
        let h = self.anim(key ^ 2, hover as u32 as f32, 22.0);
        if a > 0.01 || h > 0.01 {
            self.s.round(r, 7.0, THEME.hover.mix(THEME.sel, a).alpha(a.max(h)));
        }
        let l = self.t.line(label, st);
        self.s.text(self.t, &l, r.x + (r.w - l.width) * 0.5, r.y + (r.h - l.height) * 0.5);
        (fired, r.w)
    }

    /// Título de página numa faixa de 56 pontos.
    pub fn head(&mut self, x: f32, y: f32, title: &str) -> f32 {
        self.text(title, ui_style(15.0).weight(600), x, y + 18.0)
    }

    /// Link de texto com sublinhado que cresce.
    pub fn link(&mut self, key: Id, x: f32, y: f32, label: &str, style: TextStyle) -> (bool, f32) {
        let l = self.t.line(label, style);
        let r = Rect::new(x, y, l.width, l.height);
        let fired = self.pressed(key, r);
        let a = {
            let h = self.hover(r);
            self.anim(key ^ 1, h as u32 as f32, 20.0)
        };
        self.s.text(self.t, &l, x, y);
        self.s.fill(Rect::new(x, y + l.height - 1.0, l.width * a, 1.0), style.color);
        (fired, l.width)
    }

    /// Lista rolável: devolve o deslocamento; o chamador desenha só o que aparece.
    pub fn scroll(&mut self, key: Id, r: Rect, content: f32) -> f32 {
        let max = (content - r.h).max(0.0);
        let hover = self.hover(r);
        let wheel = self.st.wheel.1;
        let v = self.st.scrolls.entry(key).or_insert(0.0);
        if hover {
            *v -= wheel;
        }
        *v = v.clamp(0.0, max);
        let off = *v;
        if max > 0.0 {
            let h = (r.h * r.h / content).max(28.0);
            let y = r.y + (r.h - h) * (off / max);
            self.s.round(Rect::new(r.right() - 4.0, y, 3.0, h), 1.5, THEME.mute.alpha(0.35));
        }
        off
    }

    pub fn set_scroll(&mut self, key: Id, v: f32) {
        self.st.scrolls.insert(key, v);
    }

    pub fn section(&mut self, text: &str, x: f32, y: f32) {
        self.text(text, ui_style(12.0).weight(500).color(THEME.mute), x, y);
    }

    /// O feixe, marca do produto. `fine` usa cinco fios; pequeno, três mais grossos.
    pub fn mark(&mut self, cx: f32, cy: f32, size: f32, color: Color) {
        let fine = size >= 32.0;
        let k = size / 64.0;
        const FIVE: [[[f32; 2]; 4]; 5] = [[[10.0, 12.0], [27.0, 32.0], [37.0, 32.0], [54.0, 12.0]], [[7.0, 23.0], [25.0, 32.0], [39.0, 32.0], [57.0, 23.0]], [[6.0, 32.0], [6.0, 32.0], [58.0, 32.0], [58.0, 32.0]], [[7.0, 41.0], [25.0, 32.0], [39.0, 32.0], [57.0, 41.0]], [[10.0, 52.0], [27.0, 32.0], [37.0, 32.0], [54.0, 52.0]]];
        const THREE: [[[f32; 2]; 4]; 3] = [[[10.0, 10.0], [27.0, 32.0], [37.0, 32.0], [54.0, 10.0]], [[6.0, 32.0], [6.0, 32.0], [58.0, 32.0], [58.0, 32.0]], [[10.0, 54.0], [27.0, 32.0], [37.0, 32.0], [54.0, 54.0]]];
        let (paths, width): (&[[[f32; 2]; 4]], f32) = if fine { (&FIVE, 3.8 * k) } else { (&THREE, 7.0 * k) };
        for c in paths {
            let mut last = None;
            for i in 0..=14 {
                let t = i as f32 / 14.0;
                let u = 1.0 - t;
                let b = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                let (x, y) = (b[0] * c[0][0] + b[1] * c[1][0] + b[2] * c[2][0] + b[3] * c[3][0], b[0] * c[0][1] + b[1] * c[1][1] + b[2] * c[2][1] + b[3] * c[3][1]);
                // Girado 90°: os fios sobem e descem, como no ícone.
                let p = [cx + (32.0 - y) * k, cy + (x - 32.0) * k];
                if let Some(q) = last {
                    self.s.seg(q, p, width, color);
                }
                last = Some(p);
            }
        }
    }

    /// Símbolo de cada tipo de memória, o mesmo em todas as telas.
    pub fn glyph(&mut self, kind: u8, cx: f32, cy: f32, r: f32, c: Color) {
        match kind {
            1 => {
                let h = r * 1.5;
                let (top, l, rr) = ([cx, cy - h * 0.62], [cx - r, cy + h * 0.38], [cx + r, cy + h * 0.38]);
                self.s.seg(top, l, 1.3, c);
                self.s.seg(l, rr, 1.3, c);
                self.s.seg(rr, top, 1.3, c);
            }
            2 => {
                let (t, b, l, rr) = ([cx, cy - r], [cx, cy + r], [cx - r, cy], [cx + r, cy]);
                self.s.seg(t, rr, 1.3, c);
                self.s.seg(rr, b, 1.3, c);
                self.s.seg(b, l, 1.3, c);
                self.s.seg(l, t, 1.3, c);
            }
            3 => self.s.panel(Rect::new(cx - r, cy - r, r * 2.0, r * 2.0), r, Color::CLEAR, c, 1.3),
            4 => {
                self.s.seg([cx - r, cy], [cx + r, cy], 1.3, c);
                self.s.seg([cx, cy - r], [cx, cy + r], 1.3, c);
            }
            5 => self.s.panel(Rect::new(cx - r * 0.85, cy - r * 0.85, r * 1.7, r * 1.7), 0.5, Color::CLEAR, c, 1.3),
            _ => self.s.round(Rect::new(cx - r * 0.6, cy - r * 0.6, r * 1.2, r * 1.2), r, c),
        }
    }
}
