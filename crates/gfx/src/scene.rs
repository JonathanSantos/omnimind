use crate::text::{ATLAS_SIZE, Layout, TextEngine};
use crate::Color;
use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }
    pub fn inset(&self, d: f32) -> Rect {
        Rect::new(self.x + d, self.y + d, (self.w - d * 2.0).max(0.0), (self.h - d * 2.0).max(0.0))
    }
    pub fn pad(&self, l: f32, t: f32, r: f32, b: f32) -> Rect {
        Rect::new(self.x + l, self.y + t, (self.w - l - r).max(0.0), (self.h - t - b).max(0.0))
    }
    pub fn intersect(&self, o: &Rect) -> Rect {
        let x = self.x.max(o.x);
        let y = self.y.max(o.y);
        Rect::new(x, y, (self.right().min(o.right()) - x).max(0.0), (self.bottom().min(o.bottom()) - y).max(0.0))
    }
    pub fn take_left(&mut self, w: f32) -> Rect {
        let r = Rect::new(self.x, self.y, w, self.h);
        self.x += w;
        self.w = (self.w - w).max(0.0);
        r
    }
    pub fn take_right(&mut self, w: f32) -> Rect {
        self.w = (self.w - w).max(0.0);
        Rect::new(self.x + self.w, self.y, w, self.h)
    }
    pub fn take_top(&mut self, h: f32) -> Rect {
        let r = Rect::new(self.x, self.y, self.w, h);
        self.y += h;
        self.h = (self.h - h).max(0.0);
        r
    }
    pub fn take_bottom(&mut self, h: f32) -> Rect {
        self.h = (self.h - h).max(0.0);
        Rect::new(self.x, self.y + self.h, self.w, h)
    }
}

/// Retângulo arredondado instanciado. Gradiente, borda e sombra saem do mesmo SDF.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct RectPrim {
    pub bounds: [f32; 4],
    pub clip: [f32; 4],
    pub fill: [f32; 4],
    pub fill2: [f32; 4],
    pub border: [f32; 4],
    /// raio, largura da borda, desfoque, ângulo do gradiente em radianos
    pub params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct GlyphPrim {
    pub bounds: [f32; 4],
    pub uv: [f32; 4],
    pub clip: [f32; 4],
    pub color: [f32; 4],
    pub kind: f32,
    pub _p: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(crate) struct SegPrim {
    pub ab: [f32; 4],
    pub clip: [f32; 4],
    pub color: [f32; 4],
    pub color2: [f32; 4],
    pub width: f32,
    pub _p: [f32; 3],
}

#[derive(Clone, Debug)]
pub(crate) enum Op {
    Rects(std::ops::Range<u32>),
    Glyphs(std::ops::Range<u32>),
    Segs(std::ops::Range<u32>),
    Image { image: usize, rect: Rect, clip: Rect, tint: Color },
}

const NO_CLIP: Rect = Rect::new(-1e6, -1e6, 2e6, 2e6);

#[derive(Default)]
pub struct Scene {
    pub(crate) rects: Vec<RectPrim>,
    pub(crate) glyphs: Vec<GlyphPrim>,
    pub(crate) segs: Vec<SegPrim>,
    pub(crate) ops: Vec<Op>,
    clips: Vec<Rect>,
}

fn v4(r: Rect) -> [f32; 4] {
    [r.x, r.y, r.w, r.h]
}

impl Scene {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.rects.clear();
        self.glyphs.clear();
        self.segs.clear();
        self.ops.clear();
        self.clips.clear();
    }

    pub fn clip(&self) -> Rect {
        self.clips.last().copied().unwrap_or(NO_CLIP)
    }

    pub fn push_clip(&mut self, r: Rect) {
        let c = self.clip().intersect(&r);
        self.clips.push(c);
    }

    pub fn pop_clip(&mut self) {
        self.clips.pop();
    }

    pub fn stats(&self) -> (usize, usize, usize, usize) {
        (self.rects.len(), self.glyphs.len(), self.segs.len(), self.ops.len())
    }

    pub fn prim(&mut self, mut p: RectPrim) {
        p.clip = v4(self.clip());
        let i = self.rects.len() as u32;
        self.rects.push(p);
        match self.ops.last_mut() {
            Some(Op::Rects(r)) => r.end = i + 1,
            _ => self.ops.push(Op::Rects(i..i + 1)),
        }
    }

    pub fn fill(&mut self, r: Rect, c: Color) {
        self.round(r, 0.0, c);
    }

    pub fn round(&mut self, r: Rect, radius: f32, c: Color) {
        let fill = c.premul();
        self.prim(RectPrim {
            bounds: v4(r),
            clip: [0.0; 4],
            fill,
            fill2: fill,
            border: [0.0; 4],
            params: [radius, 0.0, 0.0, 0.0],
        });
    }

    pub fn panel(&mut self, r: Rect, radius: f32, fill: Color, border: Color, width: f32) {
        self.prim(RectPrim {
            bounds: v4(r),
            clip: [0.0; 4],
            fill: fill.premul(),
            fill2: fill.premul(),
            border: border.premul(),
            params: [radius, width, 0.0, 0.0],
        });
    }

    pub fn gradient(&mut self, r: Rect, radius: f32, a: Color, b: Color, angle: f32) {
        self.prim(RectPrim {
            bounds: v4(r),
            clip: [0.0; 4],
            fill: a.premul(),
            fill2: b.premul(),
            border: [0.0; 4],
            params: [radius, 0.0, 0.0, angle],
        });
    }

    pub fn shadow(&mut self, r: Rect, radius: f32, blur: f32, c: Color) {
        self.prim(RectPrim {
            bounds: v4(r),
            clip: [0.0; 4],
            fill: c.premul(),
            fill2: c.premul(),
            border: [0.0; 4],
            params: [radius, 0.0, blur, 0.0],
        });
    }

    pub fn seg(&mut self, a: [f32; 2], b: [f32; 2], width: f32, c: Color) {
        self.seg2(a, b, width, c, c);
    }

    pub fn seg2(&mut self, a: [f32; 2], b: [f32; 2], width: f32, c: Color, c2: Color) {
        let i = self.segs.len() as u32;
        self.segs.push(SegPrim {
            ab: [a[0], a[1], b[0], b[1]],
            clip: v4(self.clip()),
            color: c.premul(),
            color2: c2.premul(),
            width,
            _p: [0.0; 3],
        });
        match self.ops.last_mut() {
            Some(Op::Segs(r)) => r.end = i + 1,
            _ => self.ops.push(Op::Segs(i..i + 1)),
        }
    }

    pub fn image(&mut self, image: usize, rect: Rect, tint: Color) {
        let clip = self.clip();
        self.ops.push(Op::Image { image, rect, clip, tint });
    }

    /// Desenha um parágrafo com canto superior esquerdo em (x, y).
    pub fn text(&mut self, t: &mut TextEngine, l: &Layout, x: f32, y: f32) {
        self.text_tinted(t, l, x, y, None, 1.0);
    }

    pub fn text_tinted(&mut self, t: &mut TextEngine, l: &Layout, x: f32, y: f32, tint: Option<Color>, alpha: f32) {
        let clip = self.clip();
        // Fora do recorte: nada a fazer, e nada a rasterizar.
        if y > clip.bottom() || y + l.height < clip.y || x > clip.right() {
            return;
        }
        let scale = t.scale;
        let start = self.glyphs.len() as u32;
        let cv = v4(clip);
        let atlas = ATLAS_SIZE as f32;
        for line in &l.lines {
            if y + line.top > clip.bottom() || y + line.top + line.height < clip.y {
                continue;
            }
            for g in &l.glyphs[line.glyphs.clone()] {
                let span = g.metadata.min(l.spans.len().saturating_sub(1));
                let mut c = tint.unwrap_or_else(|| l.spans.get(span).map(|s| s.1).unwrap_or(Color::CLEAR));
                c.a *= alpha;
                let p = g.physical((x * scale, y * scale), scale);
                let Some(s) = t.slot(p.cache_key) else { continue };
                let (w, h) = (s.w as f32, s.h as f32);
                let px = (p.x + s.left as i32) as f32;
                let py = (p.y - s.top as i32) as f32;
                self.glyphs.push(GlyphPrim {
                    bounds: [px / scale, py / scale, w / scale, h / scale],
                    uv: [s.x as f32 / atlas, s.y as f32 / atlas, w / atlas, h / atlas],
                    clip: cv,
                    color: c.premul(),
                    kind: s.color as u32 as f32,
                    _p: [0.0; 3],
                });
            }
        }
        let end = self.glyphs.len() as u32;
        if end == start {
            return;
        }
        match self.ops.last_mut() {
            Some(Op::Glyphs(r)) => r.end = end,
            _ => self.ops.push(Op::Glyphs(start..end)),
        }
    }
}
