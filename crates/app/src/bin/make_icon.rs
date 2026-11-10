//! Desenha o ícone Feixe em CPU, com amostragem múltipla. `icon-1024.png` tem cinco fios;
//! `icon-small.png`, para 16 e 32 pontos, tem três fios mais grossos.
use gfx::png;

const FIVE: [[[f32; 2]; 4]; 5] = [[[10.0, 12.0], [27.0, 32.0], [37.0, 32.0], [54.0, 12.0]], [[7.0, 23.0], [25.0, 32.0], [39.0, 32.0], [57.0, 23.0]], [[6.0, 32.0], [6.0, 32.0], [58.0, 32.0], [58.0, 32.0]], [[7.0, 41.0], [25.0, 32.0], [39.0, 32.0], [57.0, 41.0]], [[10.0, 52.0], [27.0, 32.0], [37.0, 32.0], [54.0, 52.0]]];
const THREE: [[[f32; 2]; 4]; 3] = [[[10.0, 10.0], [27.0, 32.0], [37.0, 32.0], [54.0, 10.0]], [[6.0, 32.0], [6.0, 32.0], [58.0, 32.0], [58.0, 32.0]], [[10.0, 54.0], [27.0, 32.0], [37.0, 32.0], [54.0, 54.0]]];
const SIZE: usize = 1024;
const TILE: [f32; 3] = [0.09, 0.09, 0.102];
const INK: [f32; 3] = [0.949, 0.949, 0.953];

fn seg_dist(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (bax, bay) = (b[0] - a[0], b[1] - a[1]);
    let t = (((p[0] - a[0]) * bax + (p[1] - a[1]) * bay) / (bax * bax + bay * bay).max(1e-6)).clamp(0.0, 1.0);
    (p[0] - a[0] - bax * t).hypot(p[1] - a[1] - bay * t)
}

fn sampled(paths: &[[[f32; 2]; 4]], margin: f32) -> Vec<Vec<[f32; 2]>> {
    // O desenho de 64 unidades ocupa o miolo do azulejo, girado 90° como na marca.
    let k = (SIZE as f32 * (1.0 - margin * 2.0)) / 64.0;
    let o = SIZE as f32 * 0.5;
    paths
        .iter()
        .map(|c| {
            (0..=48)
                .map(|i| {
                    let t = i as f32 / 48.0;
                    let u = 1.0 - t;
                    let b = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                    let (x, y) = (b[0] * c[0][0] + b[1] * c[1][0] + b[2] * c[2][0] + b[3] * c[3][0], b[0] * c[0][1] + b[1] * c[1][1] + b[2] * c[2][1] + b[3] * c[3][1]);
                    [o + (32.0 - y) * k, o + (x - 32.0) * k]
                })
                .collect()
        })
        .collect()
}

fn draw(paths: &[[[f32; 2]; 4]], stroke: f32, margin: f32, out: &str) {
    let lines = sampled(paths, margin);
    let half = stroke * (SIZE as f32 * (1.0 - margin * 2.0)) / 64.0 * 0.5;
    // Superelipse do macOS, com a margem que o sistema espera.
    let shape = 412.0f32;
    let mut px = vec![0u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
            let (u, v) = ((cx - 512.0) / shape, (cy - 512.0) / shape);
            let e = (u.abs().powi(5) + v.abs().powi(5)).powf(0.2);
            let a = ((1.0 - e) * shape + 0.5).clamp(0.0, 1.0);
            if a == 0.0 {
                continue;
            }
            let mut d = f32::MAX;
            for l in &lines {
                for w in l.windows(2) {
                    d = d.min(seg_dist([cx, cy], w[0], w[1]));
                }
            }
            let ink = (half - d + 0.5).clamp(0.0, 1.0);
            let at = (y * SIZE + x) * 4;
            for c in 0..3 {
                px[at + c] = ((TILE[c] + (INK[c] - TILE[c]) * ink) * 255.0) as u8;
            }
            px[at + 3] = (a * 255.0) as u8;
        }
    }
    let f = std::fs::File::create(out).expect("file");
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), SIZE as u32, SIZE as u32);
    e.set_color(png::ColorType::Rgba);
    e.write_header().unwrap().write_image_data(&px).unwrap();
}

fn main() {
    draw(&FIVE, 3.8, 0.2, "assets/icon/icon-1024.png");
    draw(&THREE, 7.0, 0.16, "assets/icon/icon-small.png");
}
