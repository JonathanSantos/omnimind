use crate::Color;
use cosmic_text::{
    Attrs, Buffer, CacheKey, Family as CtFamily, FontSystem, Hinting, LayoutGlyph, LetterSpacing,
    Metrics, Shaping, Stretch, Style, SwashCache, SwashContent, Weight, Wrap, fontdb,
};
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

pub const ATLAS_SIZE: u32 = 2048;
const LAYOUT_CACHE_MAX: usize = 6000;

pub type Family = &'static str;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub family: Family,
    pub size: f32,
    pub weight: u16,
    pub italic: bool,
    pub color: Color,
    /// Espaçamento entre letras, em em.
    pub tracking: f32,
    /// Altura de linha como múltiplo do tamanho.
    pub leading: f32,
}

impl TextStyle {
    pub const fn new(family: Family, size: f32, color: Color) -> Self {
        Self { family, size, weight: 400, italic: false, color, tracking: 0.0, leading: 1.4 }
    }
    pub const fn weight(mut self, w: u16) -> Self {
        self.weight = w;
        self
    }
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
    pub const fn color(mut self, c: Color) -> Self {
        self.color = c;
        self
    }
    pub const fn tracking(mut self, t: f32) -> Self {
        self.tracking = t;
        self
    }
    pub const fn leading(mut self, l: f32) -> Self {
        self.leading = l;
        self
    }
    pub const fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
    fn key(&self, h: &mut FxHasher) {
        self.family.hash(h);
        self.size.to_bits().hash(h);
        self.weight.hash(h);
        self.italic.hash(h);
        self.tracking.to_bits().hash(h);
        self.leading.to_bits().hash(h);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Span<'a> {
    pub text: &'a str,
    pub style: TextStyle,
}

impl<'a> Span<'a> {
    pub fn new(text: &'a str, style: TextStyle) -> Self {
        Self { text, style }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug)]
pub struct Line {
    pub top: f32,
    pub baseline: f32,
    pub height: f32,
    pub width: f32,
    /// Faixa de glifos desta linha visual dentro de `Layout::glyphs`.
    pub glyphs: std::ops::Range<usize>,
    /// Faixa de bytes do texto coberta pela linha visual.
    pub bytes: std::ops::Range<usize>,
}

/// Parágrafo moldado, em pontos lógicos. Imutável e compartilhável.
#[derive(Debug)]
pub struct Layout {
    pub glyphs: Vec<LayoutGlyph>,
    pub lines: Vec<Line>,
    pub spans: Vec<(std::ops::Range<usize>, Color)>,
    pub width: f32,
    pub height: f32,
    pub len: usize,
}

impl Layout {
    fn line_of(&self, byte: usize) -> usize {
        for (i, l) in self.lines.iter().enumerate() {
            if byte < l.bytes.end || i + 1 == self.lines.len() {
                return i;
            }
        }
        0
    }

    /// Posição do cursor antes do byte dado: (x, topo, altura).
    pub fn caret(&self, byte: usize) -> (f32, f32, f32) {
        let Some(first) = self.lines.first() else { return (0.0, 0.0, 0.0) };
        let li = self.line_of(byte);
        let line = self.lines.get(li).unwrap_or(first);
        let mut x = 0.0;
        for g in &self.glyphs[line.glyphs.clone()] {
            if byte <= g.start {
                return (g.x, line.top, line.height);
            }
            if byte < g.end {
                let t = (byte - g.start) as f32 / (g.end - g.start) as f32;
                return (g.x + g.w * t, line.top, line.height);
            }
            x = g.x + g.w;
        }
        (x, line.top, line.height)
    }

    /// Byte mais próximo do ponto (x, y) relativo ao topo do parágrafo.
    pub fn hit(&self, x: f32, y: f32) -> usize {
        let Some(mut line) = self.lines.first() else { return 0 };
        for l in &self.lines {
            if y >= l.top {
                line = l;
            }
        }
        let mut best = line.bytes.end;
        for g in &self.glyphs[line.glyphs.clone()] {
            if x < g.x + g.w * 0.5 {
                return g.start;
            }
            best = g.end;
        }
        best
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Slot {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub left: i16,
    pub top: i16,
    pub color: bool,
}

pub(crate) struct Upload {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub color: bool,
    pub data: Vec<u8>,
}

#[derive(Default)]
struct Shelf {
    x: u32,
    y: u32,
    row: u32,
}

impl Shelf {
    fn alloc(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if self.x + w + 1 >= ATLAS_SIZE {
            self.x = 0;
            self.y += self.row + 1;
            self.row = 0;
        }
        if self.y + h + 1 >= ATLAS_SIZE {
            return None;
        }
        let at = (self.x, self.y);
        self.x += w + 1;
        self.row = self.row.max(h);
        Some(at)
    }
}

pub struct TextEngine {
    fonts: FontSystem,
    swash: SwashCache,
    layouts: FxHashMap<u64, (Arc<Layout>, u64)>,
    slots: FxHashMap<CacheKey, Option<Slot>>,
    mask: Shelf,
    color: Shelf,
    pub(crate) uploads: Vec<Upload>,
    pub(crate) reset: bool,
    tick: u64,
    pub scale: f32,
    /// Contadores do quadro, zerados por quem mede.
    pub stat_raster: u32,
    pub stat_shaped: u32,
    embedded: Vec<&'static [u8]>,
    fallback: Option<Receiver<fontdb::Database>>,
}

impl TextEngine {
    /// Só as fontes embutidas: varrer as ~800 do sistema custa 170 ms na abertura.
    pub fn new(fonts: &[&'static [u8]], scale: f32) -> Self {
        let embedded = fonts.to_vec();
        let mut db = fontdb::Database::new();
        for f in fonts {
            db.load_font_source(fontdb::Source::Binary(Arc::new(*f)));
        }
        let fonts = FontSystem::new_with_locale_and_db("pt-BR".into(), db);
        Self {
            embedded,
            fallback: None,
            fonts,
            swash: SwashCache::new(),
            layouts: FxHashMap::default(),
            slots: FxHashMap::default(),
            mask: Shelf::default(),
            color: Shelf::default(),
            uploads: Vec::new(),
            reset: false,
            tick: 0,
            scale,
            stat_raster: 0,
            stat_shaped: 0,
        }
    }

    /// Varre as fontes do sistema fora do quadro. Emoji, CJK e símbolos chegam logo depois.
    pub fn with_system_fallback(mut self) -> Self {
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("fonts".into())
            .spawn(move || {
                let mut db = fontdb::Database::new();
                db.load_system_fonts();
                let _ = tx.send(db);
            })
            .expect("font thread");
        self.fallback = Some(rx);
        self
    }

    /// Adota as fontes do sistema quando prontas. IDs mudam, então os caches recomeçam.
    pub fn poll_fallback(&mut self) -> bool {
        let Some(rx) = &self.fallback else { return false };
        let Ok(mut db) = rx.try_recv() else { return false };
        self.fallback = None;
        for f in &self.embedded {
            db.load_font_source(fontdb::Source::Binary(Arc::new(*f)));
        }
        self.fonts = FontSystem::new_with_locale_and_db("pt-BR".into(), db);
        self.layouts.clear();
        self.clear_atlas();
        true
    }

    pub fn set_scale(&mut self, scale: f32) {
        if scale != self.scale {
            self.scale = scale;
            self.clear_atlas();
        }
    }

    pub fn clear_atlas(&mut self) {
        self.slots.clear();
        self.mask = Shelf::default();
        self.color = Shelf::default();
        self.uploads.clear();
        self.reset = true;
    }

    pub fn layout(&mut self, spans: &[Span<'_>], width: Option<f32>, align: Align) -> Arc<Layout> {
        let mut h = FxHasher::default();
        for s in spans {
            s.text.hash(&mut h);
            s.style.key(&mut h);
            s.style.color.r.to_bits().hash(&mut h);
            s.style.color.g.to_bits().hash(&mut h);
            s.style.color.b.to_bits().hash(&mut h);
            s.style.color.a.to_bits().hash(&mut h);
        }
        width.map(f32::to_bits).hash(&mut h);
        align.hash(&mut h);
        let key = h.finish();
        self.tick += 1;
        if let Some((l, seen)) = self.layouts.get_mut(&key) {
            *seen = self.tick;
            return l.clone();
        }
        self.stat_shaped += 1;
        let layout = Arc::new(self.shape(spans, width, align));
        if self.layouts.len() >= LAYOUT_CACHE_MAX {
            let cut = self.tick.saturating_sub(LAYOUT_CACHE_MAX as u64 / 2);
            self.layouts.retain(|_, (_, seen)| *seen >= cut);
        }
        self.layouts.insert(key, (layout.clone(), self.tick));
        layout
    }

    pub fn line(&mut self, text: &str, style: TextStyle) -> Arc<Layout> {
        self.layout(&[Span::new(text, style)], None, Align::Left)
    }

    fn shape(&mut self, spans: &[Span<'_>], width: Option<f32>, align: Align) -> Layout {
        let first = spans.first().map(|s| s.style).unwrap_or(TextStyle::new("", 14.0, Color::CLEAR));
        let metrics = Metrics::new(first.size, (first.size * first.leading).round());
        let mut buffer = Buffer::new(&mut self.fonts, metrics);
        buffer.set_hinting(Hinting::Disabled);
        buffer.set_wrap(if width.is_some() { Wrap::WordOrGlyph } else { Wrap::None });
        buffer.set_size(width, None);
        let attrs = |s: &TextStyle, i: usize| {
            let mut a = Attrs::new()
                .family(CtFamily::Name(s.family))
                .weight(Weight(s.weight))
                .stretch(Stretch::Normal)
                .style(if s.italic { Style::Italic } else { Style::Normal })
                .metadata(i)
                .metrics(Metrics::new(s.size, (s.size * s.leading).round()));
            if s.tracking != 0.0 {
                a.letter_spacing_opt = Some(LetterSpacing(s.tracking));
            }
            a
        };
        let default = attrs(&first, 0);
        buffer.set_rich_text(
            spans.iter().enumerate().map(|(i, s)| (s.text, attrs(&s.style, i))),
            &default,
            Shaping::Advanced,
            Some(match align {
                Align::Left => cosmic_text::Align::Left,
                Align::Center => cosmic_text::Align::Center,
                Align::Right => cosmic_text::Align::Right,
            }),
        );
        buffer.shape_until_scroll(&mut self.fonts, false);

        let mut glyphs = Vec::new();
        let mut lines = Vec::new();
        let mut max_w = 0f32;
        let mut height = 0f32;
        // Offset em bytes de cada linha do buffer (texto com '\n' vira várias BufferLine).
        let mut line_starts = vec![0usize];
        let total: String = spans.iter().map(|s| s.text).collect();
        for (i, b) in total.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        for run in buffer.layout_runs() {
            let base = line_starts.get(run.line_i).copied().unwrap_or(0);
            let start = glyphs.len();
            for g in run.glyphs {
                let mut g = g.clone();
                g.start += base;
                g.end += base;
                g.y = run.line_y;
                glyphs.push(g);
            }
            let bytes = match (run.glyphs.first(), run.glyphs.last()) {
                (Some(a), Some(z)) => a.start.min(z.start) + base..a.end.max(z.end) + base,
                _ => base..base,
            };
            lines.push(Line {
                top: run.line_top,
                baseline: run.line_y,
                height: run.line_height,
                width: run.line_w,
                glyphs: start..glyphs.len(),
                bytes,
            });
            max_w = max_w.max(run.line_w);
            height = height.max(run.line_top + run.line_height);
        }
        // Linhas visuais são contíguas em bytes: o fim de uma é o começo da próxima.
        for i in 1..lines.len() {
            let start = lines[i].bytes.start;
            if start > lines[i - 1].bytes.end {
                lines[i - 1].bytes.end = start;
            }
        }
        if let Some(l) = lines.last_mut() {
            l.bytes.end = total.len();
        }
        let mut at = 0;
        let spans = spans
            .iter()
            .map(|s| {
                let r = at..at + s.text.len();
                at = r.end;
                (r, s.style.color)
            })
            .collect();
        Layout { glyphs, lines, spans, width: max_w, height, len: total.len() }
    }

    pub(crate) fn slot(&mut self, key: CacheKey) -> Option<Slot> {
        if let Some(s) = self.slots.get(&key) {
            return *s;
        }
        let slot = self.rasterize(key);
        self.slots.insert(key, slot);
        slot
    }

    fn rasterize(&mut self, key: CacheKey) -> Option<Slot> {
        self.stat_raster += 1;
        let img = self.swash.get_image_uncached(&mut self.fonts, key)?;
        let (w, h) = (img.placement.width, img.placement.height);
        if w == 0 || h == 0 {
            return None;
        }
        let color = img.content == SwashContent::Color;
        let shelf = if color { &mut self.color } else { &mut self.mask };
        let Some((x, y)) = shelf.alloc(w, h) else {
            self.clear_atlas();
            return None;
        };
        self.uploads.push(Upload { x, y, w, h, color, data: img.data });
        Some(Slot {
            x: x as u16,
            y: y as u16,
            w: w as u16,
            h: h as u16,
            left: img.placement.left as i16,
            top: img.placement.top as i16,
            color,
        })
    }
}
