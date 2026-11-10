//! Linhas viram fitas de triângulos: é o que permite milhares delas com espessura e sem custo por quadro.

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct LineVertex {
    pub pos: [f32; 2],
    pub normal: [f32; 2],
    pub side: f32,
    pub along: f32,
    pub delay: f32,
    pub width: f32,
    pub alpha: f32,
    pub themes: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Dot {
    pub pos: [f32; 2],
    pub size: f32,
    pub delay: f32,
    pub alpha: f32,
    pub theme: f32,
    /// 0 é disco cheio, 1 é anel. Memória e nota: a forma diz o tipo, não só a cor.
    pub kind: f32,
}

pub struct Stroke {
    pub width: f32,
    pub alpha: f32,
    pub delay: f32,
    pub themes: [f32; 2],
}

#[derive(Default)]
pub struct LineMesh {
    pub vertices: Vec<LineVertex>,
    pub indices: Vec<u32>,
}

impl LineMesh {
    pub fn pair(&mut self, pos: [f32; 2], normal: [f32; 2], along: f32, s: &Stroke) -> u32 {
        let i = self.vertices.len() as u32;
        let v = LineVertex { pos, normal, side: 1.0, along, delay: s.delay, width: s.width, alpha: s.alpha, themes: s.themes };
        self.vertices.push(v);
        self.vertices.push(LineVertex { side: -1.0, ..v });
        i
    }

    pub fn quad(&mut self, a: u32, b: u32) {
        self.indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
    }

    /// Fita por uma polilinha. Devolve o trecho de índices.
    pub fn ribbon(&mut self, pts: &[[f32; 2]], s: &Stroke) -> (u32, u32) {
        let start = self.indices.len() as u32;
        let n = pts.len();
        let mut prev = None;
        for k in 0..n {
            let (p0, p1) = (pts[k.saturating_sub(1)], pts[(k + 1).min(n - 1)]);
            let (mut tx, mut ty) = (p1[0] - p0[0], p1[1] - p0[1]);
            let len = tx.hypot(ty).max(1e-5);
            tx /= len;
            ty /= len;
            let a = self.pair(pts[k], [-ty, tx], k as f32 / (n - 1).max(1) as f32, s);
            if let Some(b) = prev {
                self.quad(b, a);
            }
            prev = Some(a);
        }
        (start, self.indices.len() as u32)
    }
}

#[derive(Default)]
pub struct DotList {
    pub dots: Vec<Dot>,
}

impl DotList {
    pub fn add(&mut self, pos: [f32; 2], size: f32, delay: f32, alpha: f32, theme: u8, ring: bool) -> u32 {
        self.dots.push(Dot { pos, size, delay, alpha, theme: theme as f32, kind: ring as u32 as f32 });
        self.dots.len() as u32 - 1
    }
}
