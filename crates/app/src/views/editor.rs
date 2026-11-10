//! Editor com preview ao vivo: a sintaxe some fora da linha ativa, e só as linhas
//! visíveis são moldadas. O cache de layout do `TextEngine` faz o resto.

use crate::theme::{body, MONO, THEME};
use ed::md::{self, Carry, Inline, Kind, Mark};
use ed::Buffer;
use gfx::{Align, Color, Layout, Rect, Scene, Span, TextEngine, TextStyle};
use std::sync::Arc;

const COLUMN: f32 = 680.0;

/// Mapa fonte → exibição de uma linha. Cada trecho é contíguo nos dois lados.
#[derive(Clone, Debug, Default)]
struct Map(Vec<(usize, usize, usize)>);

impl Map {
    fn to_display(&self, src: usize) -> usize {
        let mut last = 0;
        for &(s, d, n) in &self.0 {
            if src < s {
                return d;
            }
            if src <= s + n {
                return d + (src - s);
            }
            last = d + n;
        }
        last
    }
    fn to_source(&self, disp: usize) -> usize {
        let mut last = 0;
        for &(s, d, n) in &self.0 {
            if disp <= d + n {
                return s + disp.saturating_sub(d);
            }
            last = s + n;
        }
        last
    }
}

struct LineView {
    layout: Arc<Layout>,
    map: Map,
    kind: Kind,
    pad_top: f32,
    indent: f32,
}

pub struct EditorView {
    pub buf: Buffer,
    pub scroll: f32,
    carry: Vec<Carry>,
    kinds: Vec<Kind>,
    heights: Vec<f32>,
    seen_version: u64,
    seen_width: f32,
    last_rect: Rect,
    /// Topo (y de documento) e parâmetros da última geometria desenhada.
    tops: Vec<f32>,
    /// Posição do cursor na tela, para ancorar a janela de composição do sistema.
    pub caret_screen: (f32, f32, f32),
    /// Alvos de wikilink que não existem, em minúsculas: aparecem como pendentes.
    pub missing: std::collections::HashSet<String>,
}

fn style_for(kind: Kind) -> TextStyle {
    let b = body();
    match kind {
        Kind::Heading(1) => b.size(34.0).weight(700).leading(1.2),
        Kind::Heading(2) => b.size(26.0).weight(700).leading(1.25),
        Kind::Heading(3) => b.size(21.0).weight(700).leading(1.3),
        Kind::Heading(_) => b.size(18.0).weight(700),
        Kind::Quote => b.color(THEME.sub).italic(),
        Kind::Code | Kind::Fence | Kind::Table => {
            TextStyle::new(MONO, 14.0, THEME.ink).leading(1.5)
        }
        Kind::Front => TextStyle::new(MONO, 13.0, THEME.sub).leading(1.5),
        _ => b,
    }
}

/// Monta os trechos de exibição de uma linha. Linha ativa mostra a sintaxe, apagada.
fn build(text: &str, kind: Kind, active: bool, missing: &std::collections::HashSet<String>) -> (Vec<(String, TextStyle)>, Map) {
    let base = style_for(kind);
    let mut out: Vec<(String, TextStyle)> = Vec::new();
    let mut map = Map::default();
    if matches!(kind, Kind::Code | Kind::Fence | Kind::Front | Kind::Rule) {
        map.0.push((0, 0, text.len()));
        return (vec![(text.to_string(), base)], map);
    }
    let prefix = md::prefix_len(text, kind).min(text.len());
    let inl: Vec<Inline> = md::inlines(&text[prefix..])
        .into_iter()
        .map(|mut i| {
            i.range = i.range.start + prefix..i.range.end + prefix;
            for s in &mut i.syntax {
                *s = s.start + prefix..s.end + prefix;
            }
            i
        })
        .collect();
    // Cortes: cada fronteira de estilo ou sintaxe vira um pedaço.
    let mut cuts = vec![0, prefix, text.len()];
    for i in &inl {
        cuts.push(i.range.start);
        cuts.push(i.range.end);
        for s in &i.syntax {
            cuts.push(s.start);
            cuts.push(s.end);
        }
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut disp = 0;
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let is_prefix = a < prefix;
        let is_syntax = is_prefix || inl.iter().any(|i| i.syntax.iter().any(|s| s.start <= a && b <= s.end));
        if is_syntax && !active {
            continue;
        }
        let mut st = base;
        if is_syntax {
            st = st.color(THEME.mute);
        }
        for i in inl.iter().filter(|i| i.range.start <= a && b <= i.range.end) {
            st = match i.mark {
                Mark::Bold => st.weight(700),
                Mark::Italic => st.italic(),
                Mark::Code => {
                    let mut s = TextStyle::new(MONO, st.size * 0.86, THEME.sub);
                    s.leading = st.leading * st.size / (st.size * 0.86);
                    s
                }
                Mark::Link | Mark::Url => st.color(THEME.acc),
                Mark::Wiki => {
                    let name = i.target.clone().map(|t| &text[t]).unwrap_or("");
                    if missing.contains(&name.to_lowercase()) { st.color(THEME.sub).italic() } else { st.color(THEME.acc) }
                }
                Mark::Tag => st.color(THEME.mute),
                Mark::Highlight => st.color(THEME.ink).weight(600),
                Mark::Strike => st.color(THEME.sub),
                Mark::Hidden => st.color(THEME.warn),
            };
        }
        let piece = &text[a..b];
        if inl.iter().any(|i| i.mark == Mark::Hidden && i.range.start == a) {
            // Invisível vira visível, sempre. Ocupa largura e não engana ninguém.
            let shown = format!("⟦U+{:04X}⟧", piece.chars().next().map(|c| c as u32).unwrap_or(0));
            map.0.push((a, disp, 0));
            disp += shown.len();
            out.push((shown, st));
            map.0.push((b, disp, 0));
            continue;
        }
        map.0.push((a, disp, b - a));
        disp += b - a;
        out.push((piece.to_string(), st));
    }
    if out.is_empty() {
        out.push((String::new(), base));
    }
    (out, map)
}

impl EditorView {
    pub fn new(text: &str) -> Self {
        Self {
            buf: Buffer::new(text),
            scroll: 0.0,
            carry: Vec::new(),
            kinds: Vec::new(),
            heights: Vec::new(),
            seen_version: u64::MAX,
            seen_width: 0.0,
            last_rect: Rect::default(),
            tops: Vec::new(),
            caret_screen: (0.0, 0.0, 0.0),
            missing: Default::default(),
        }
    }

    /// Reclassifica a partir da linha tocada e para quando o estado volta a coincidir.
    fn reclassify(&mut self) {
        let n = self.buf.len_lines();
        let grew = n != self.kinds.len();
        let from = self.buf.take_dirty().unwrap_or(0).min(n.saturating_sub(1));
        self.kinds.resize(n, Kind::Body);
        self.carry.resize(n + 1, Carry::default());
        self.heights.resize(n, 0.0);
        let mut c = self.carry[from];
        for i in from..n {
            let (k, next) = md::classify(&self.buf.line(i), i, c);
            if self.kinds[i] != k {
                self.heights[i] = 0.0;
            }
            self.heights[i] = if grew { 0.0 } else { self.heights[i] };
            self.kinds[i] = k;
            self.carry[i] = c;
            c = next;
            if !grew && i > from && self.carry[i + 1] == c {
                break;
            }
            self.carry[i + 1] = c;
        }
    }

    fn front_active(&self) -> bool {
        let (line, _) = self.buf.line_byte(self.buf.head());
        self.kinds.get(line) == Some(&Kind::Front)
    }

    fn est_height(&self, i: usize, width: f32) -> f32 {
        let st = style_for(self.kinds[i]);
        let per_line = (width / (st.size * 0.5)).max(10.0);
        let chars = self.buf.line(i).chars().count() as f32;
        let lines = (chars / per_line).ceil().max(1.0);
        (st.size * st.leading).round() * lines + pad_top(self.kinds[i])
    }

    fn line_view(&self, i: usize, text: &mut TextEngine, width: f32, active: bool, line: &str) -> LineView {
        let kind = self.kinds[i];
        // Frontmatter recolhe numa linha discreta enquanto o cursor não está nele.
        if kind == Kind::Front && !self.front_active() {
            let shown = if i == 0 { "propriedades" } else { "" };
            let st = TextStyle::new(MONO, 11.0, THEME.mute).leading(if i == 0 { 1.5 } else { 0.01 });
            let layout = text.layout(&[Span::new(shown, st)], Some(width), Align::Left);
            return LineView { layout, map: Map(vec![(0, 0, 0)]), kind, pad_top: 0.0, indent: 0.0 };
        }
        let (pieces, map) = build(line, kind, active, &self.missing);
        let spans: Vec<Span<'_>> = pieces.iter().map(|(t, s)| Span::new(t, *s)).collect();
        let indent = match kind {
            Kind::Bullet { indent, .. } | Kind::Ordered { indent } => 22.0 + indent as f32 * 8.0,
            Kind::Quote => 20.0,
            _ => 0.0,
        };
        let layout = text.layout(&spans, Some((width - indent).max(40.0)), Align::Left);
        LineView { layout, map, kind, pad_top: pad_top(kind), indent }
    }

    /// Alvo do link sob um caractere, se houver.
    pub fn link_at(&self, ch: usize) -> Option<String> {
        let (line, byte) = self.buf.line_byte(ch);
        let text = self.buf.line(line);
        md::inlines(&text).into_iter().find_map(|i| {
            let hit = i.range.contains(&byte) && matches!(i.mark, Mark::Wiki | Mark::Link | Mark::Url);
            hit.then(|| i.target.map(|t| text[t].to_string())).flatten()
        })
    }

    /// Texto digitado depois de `[[` na linha do cursor, se o link ainda estiver aberto.
    pub fn open_link_query(&self) -> Option<String> {
        let (line, byte) = self.buf.line_byte(self.buf.head());
        let text = self.buf.line(line);
        let before = text.get(..byte)?;
        let start = before.rfind("[[")?;
        let q = &before[start + 2..];
        (!q.contains("]]") && q.len() < 60).then(|| q.to_string())
    }

    /// Fecha o link em aberto com o título escolhido.
    pub fn complete_link(&mut self, title: &str) {
        let Some(q) = self.open_link_query() else { return };
        let h = self.buf.head();
        let n = q.chars().count();
        self.buf.select(h - n, h);
        self.buf.insert(&format!("{title}]]"));
    }

    /// Traz o cursor para a área visível. Usa alturas conhecidas ou estimadas.
    pub fn reveal_caret(&mut self, view_h: f32) {
        let (line, _) = self.buf.line_byte(self.buf.head());
        let Some(&top) = self.tops.get(line) else { return };
        let h = self.heights.get(line).copied().unwrap_or(24.0);
        if top < self.scroll + 40.0 {
            self.scroll = (top - 40.0).max(0.0);
        } else if top + h > self.scroll + view_h - 80.0 {
            self.scroll = top + h - view_h + 80.0;
        }
    }

    pub fn paint(&mut self, r: Rect, scene: &mut Scene, text: &mut TextEngine, focused: bool, _time: f32) {
        let width = COLUMN.min(r.w - 80.0).max(120.0);
        let left = r.x + ((r.w - width) * 0.5).floor();
        self.last_rect = r;
        if self.seen_version != self.buf.version || self.kinds.len() != self.buf.len_lines() {
            self.reclassify();
            self.seen_version = self.buf.version;
        }
        if self.seen_width != width {
            self.heights.iter_mut().for_each(|h| *h = 0.0);
            self.seen_width = width;
        }
        let n = self.kinds.len();
        for i in 0..n {
            if self.heights[i] == 0.0 {
                self.heights[i] = self.est_height(i, width);
            }
        }
        self.tops.resize(n + 1, 0.0);
        let mut y = 24.0;
        for i in 0..n {
            self.tops[i] = y;
            y += self.heights[i];
        }
        self.tops[n] = y;
        let total = y + r.h * 0.5;
        self.scroll = self.scroll.clamp(0.0, (total - r.h).max(0.0));

        let first = self.tops.partition_point(|t| *t < self.scroll).saturating_sub(1).min(n.saturating_sub(1));
        let (h_line, h_byte) = self.buf.line_byte(self.buf.head());
        let (sel_a, sel_b) = self.buf.selection();
        scene.push_clip(r);
        let mut i = first;
        let mut doc_y = self.tops[first];
        while i < n {
            let sy = r.y + doc_y - self.scroll;
            if sy > r.bottom() {
                break;
            }
            let mut line = self.buf.line(i);
            let active = focused && i == h_line;
            let composing = active && !self.buf.preedit.is_empty();
            if composing {
                line.insert_str(h_byte.min(line.len()), &self.buf.preedit);
            }
            let v = self.line_view(i, text, width, active, &line);
            let h = v.layout.height + v.pad_top;
            self.heights[i] = h;
            self.paint_line(&v, i, &line, left, sy, width, scene, text, sel_a, sel_b);
            if active {
                let end = h_byte + if composing { self.buf.preedit.len() } else { 0 };
                let disp = v.map.to_display(end);
                let (cx, ct, ch) = v.layout.caret(disp);
                let (x, y) = (left + v.indent + cx, sy + v.pad_top + ct);
                if composing {
                    // Composição sublinhada: a tecla morta aparece antes de virar acento.
                    let (x0, _, _) = v.layout.caret(v.map.to_display(h_byte));
                    scene.fill(Rect::new(left + v.indent + x0, y + ch - 3.0, (cx - x0).max(6.0), 1.5), THEME.acc);
                }
                scene.fill(Rect::new(x - 0.75, y + 2.0, 1.5, ch - 4.0), THEME.ink);
                self.caret_screen = (x, y, ch);
            }
            doc_y += h;
            i += 1;
        }
        scene.pop_clip();
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_line(
        &self, v: &LineView, i: usize, line: &str, left: f32, sy: f32, width: f32,
        scene: &mut Scene, text: &mut TextEngine, sel_a: usize, sel_b: usize,
    ) {
        let top = sy + v.pad_top;
        match v.kind {
            Kind::Rule => {
                scene.fill(Rect::new(left, top + 12.0, width, 1.0), THEME.line);
                return;
            }
            Kind::Code | Kind::Fence => {
                scene.fill(Rect::new(left - 14.0, sy, width + 28.0, v.layout.height + v.pad_top), THEME.code_bg);
            }
            Kind::Quote => {
                scene.fill(Rect::new(left + 2.0, sy, 2.0, v.layout.height), THEME.line.alpha(0.7));
            }
            Kind::Bullet { indent, task: None } => {
                scene.round(Rect::new(left + 8.0 + indent as f32 * 8.0, top + 11.0, 5.0, 5.0), 2.5, THEME.line);
            }
            Kind::Bullet { indent, task: Some(done) } => {
                let b = Rect::new(left + 3.0 + indent as f32 * 8.0, top + 7.0, 13.0, 13.0);
                scene.panel(b, 0.0, if done { THEME.line.alpha(0.2) } else { Color::CLEAR }, THEME.line.alpha(0.8), 1.0);
                if done {
                    scene.seg([b.x + 3.0, b.y + 7.0], [b.x + 5.5, b.y + 9.5], 1.5, THEME.ink);
                    scene.seg([b.x + 5.5, b.y + 9.5], [b.x + 10.0, b.y + 3.5], 1.5, THEME.ink);
                }
            }
            _ => {}
        }
        if sel_a != sel_b {
            let ls = self.buf.line_start(i);
            let le = ls + line.chars().count();
            if sel_a <= le && sel_b >= ls {
                let a = v.map.to_display(self.buf.line_byte(sel_a.max(ls)).1);
                let b = v.map.to_display(self.buf.line_byte(sel_b.min(le)).1);
                for l in &v.layout.lines {
                    let s = a.max(l.bytes.start);
                    let e = b.min(l.bytes.end);
                    if s > e || (s == e && sel_b <= le) {
                        continue;
                    }
                    let (x0, _, _) = v.layout.caret(s);
                    let (x1, _, _) = v.layout.caret(e);
                    let x1 = if sel_b > le && e == l.bytes.end { x1 + 6.0 } else { x1 };
                    scene.fill(
                        Rect::new(left + v.indent + x0, top + l.top, (x1 - x0).max(2.0), l.height),
                        THEME.select,
                    );
                }
            }
        }
        scene.text(text, &v.layout, left + v.indent, top);
    }

    /// Clique: de coordenada de tela para caractere.
    pub fn hit(&self, x: f32, y: f32, text: &mut TextEngine) -> usize {
        let r = self.last_rect;
        let width = self.seen_width;
        let left = r.x + ((r.w - width) * 0.5).floor();
        let doc = y - r.y + self.scroll;
        let i = self.tops.partition_point(|t| *t <= doc).saturating_sub(1).min(self.kinds.len().saturating_sub(1));
        let line = self.buf.line(i);
        let (h_line, _) = self.buf.line_byte(self.buf.head());
        let v = self.line_view(i, text, width, i == h_line, &line);
        let disp = v.layout.hit(x - left - v.indent, doc - self.tops[i] - v.pad_top);
        self.buf.char_at(i, v.map.to_source(disp))
    }
}

fn pad_top(kind: Kind) -> f32 {
    match kind {
        Kind::Heading(1) => 26.0,
        Kind::Heading(2) => 22.0,
        Kind::Heading(_) => 16.0,
        _ => 0.0,
    }
}
