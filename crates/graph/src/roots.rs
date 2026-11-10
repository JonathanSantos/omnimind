//! Raízes: cada tema cresce do centro por colonização de espaço (Runions, 2007).
//! Onde há mais uso o galho engrossa.

use crate::mesh::Stroke;
use crate::{dust, haze, rng, tier_of, Created, DotList, Form, Input, Item, Label, LineMesh, Scene, Span, SLOTS};
use std::f32::consts::{FRAC_PI_2, TAU};

const RX: f32 = 374.0;
const RY: f32 = 246.0;
const CY: f32 = 6.0;
const WORLD: f32 = 1000.0;

struct Attractor {
    pos: [f32; 2],
    theme: u8,
    item: i32,
    best: i32,
    best_d: f32,
    end: i32,
    alive: bool,
}

#[derive(Default)]
struct Tree {
    x: Vec<f32>,
    y: Vec<f32>,
    parent: Vec<i32>,
    theme: Vec<u8>,
}

impl Tree {
    fn add(&mut self, x: f32, y: f32, parent: i32, theme: u8) -> i32 {
        self.x.push(x);
        self.y.push(y);
        self.parent.push(parent);
        self.theme.push(theme);
        self.x.len() as i32 - 1
    }
}

fn unit(x: f32, y: f32) -> [f32; 2] {
    let l = x.hypot(y).max(1e-5);
    [x / l, y / l]
}

pub fn build(input: &Input) -> Scene {
    let total = input.nodes.len();
    let mut scene = Scene::empty(Form::Roots);
    scene.span = 1.0;
    scene.head = 0.05;
    if total == 0 {
        return scene;
    }
    let tier = tier_of(total);
    scene.tier = tier;
    scene.open_seconds = if tier == 2 { 4.2 } else { 3.4 };
    let (step, kill, reach, extra) = ([7.5, 6.0, 4.5][tier], [10.0, 8.5, 6.5][tier], [130.0, 84.0, 52.0][tier], [4, 1, 0][tier]);
    let mut count = [0usize; SLOTS];
    for x in &input.nodes {
        count[x.theme as usize] += 1;
    }
    let mut sector = [(0f32, 0f32); SLOTS];
    let mut acc = -FRAC_PI_2;
    for g in 0..SLOTS {
        sector[g] = (acc, acc + count[g] as f32 / total as f32 * TAU);
        acc = sector[g].1;
    }
    let live: Vec<u8> = (0..SLOTS as u8).filter(|&g| count[g as usize] > 0).collect();
    let mut atts: Vec<Attractor> = Vec::with_capacity(total * (1 + extra));
    let place = |g: u8, item: i32, r: &mut dyn FnMut() -> f32, atts: &mut Vec<Attractor>| {
        let s = sector[g as usize];
        let a = s.0 + (0.05 + r() * 0.9) * (s.1 - s.0);
        let d = 0.12 + r().sqrt() * 0.88;
        atts.push(Attractor { pos: [a.cos() * d * RX, CY + a.sin() * d * RY], theme: g, item, best: -1, best_d: 1e12, end: -1, alive: true });
    };
    for (i, x) in input.nodes.iter().enumerate() {
        if x.fresh {
            continue;
        }
        // Sorteio preso à identidade do nó: quem chega depois não embaralha quem já estava.
        let mut r = rng(x.key.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        place(x.theme, i as i32, &mut r, &mut atts);
        for _ in 0..extra {
            place(x.theme, -1, &mut r, &mut atts);
        }
    }
    // Um tronco de pontos leva cada tema até a sua região, mesmo os temas pequenos.
    for &g in &live {
        let a = (sector[g as usize].0 + sector[g as usize].1) * 0.5;
        let mut d = 0.08;
        while d < 0.62 {
            atts.push(Attractor { pos: [a.cos() * d * RX, CY + a.sin() * d * RY], theme: g, item: -1, best: -1, best_d: 1e12, end: -1, alive: true });
            d += 0.06;
        }
    }
    let cells = (2.0 * WORLD / reach) as usize + 2;
    let cell_of = |x: f32, y: f32| (((x + WORLD) / reach).clamp(0.0, cells as f32 - 1.0) as usize, ((y + WORLD) / reach).clamp(0.0, cells as f32 - 1.0) as usize);
    let mut grid: Vec<Vec<u32>> = vec![vec![]; cells * cells];
    for (i, a) in atts.iter().enumerate() {
        let (cx, cy) = cell_of(a.pos[0], a.pos[1]);
        grid[cx + cy * cells].push(i as u32);
    }
    let mut t = Tree::default();
    for &g in &live {
        let a = (sector[g as usize].0 + sector[g as usize].1) * 0.5;
        t.add(a.cos() * 10.0, CY + a.sin() * 10.0, -1, g);
    }
    let mut noise = rng(99);
    let mut fresh: Vec<i32> = (0..t.x.len() as i32).collect();
    let (reach2, kill2) = (reach * reach, kill * kill);
    let mut pull: Vec<[f32; 2]> = vec![];
    let mut touched: Vec<i32> = vec![];
    // Direções em que cada ponta já cresceu: dois pontos puxando em sentidos opostos
    // repetiriam o mesmo galho até o fim, e a sobreposição estoura em branco.
    let mut grew: Vec<Vec<[f32; 2]>> = vec![];
    for _ in 0..420 {
        if fresh.is_empty() {
            break;
        }
        for &s in &fresh {
            let (sx, sy, sg) = (t.x[s as usize], t.y[s as usize], t.theme[s as usize]);
            let (cx, cy) = cell_of(sx, sy);
            for gy in cy.saturating_sub(1)..(cy + 2).min(cells) {
                for gx in cx.saturating_sub(1)..(cx + 2).min(cells) {
                    for &i in &grid[gx + gy * cells] {
                        let a = &mut atts[i as usize];
                        if !a.alive || a.theme != sg {
                            continue;
                        }
                        let d = (a.pos[0] - sx).powi(2) + (a.pos[1] - sy).powi(2);
                        if d < a.best_d && d < reach2 {
                            a.best_d = d;
                            a.best = s;
                        }
                    }
                }
            }
        }
        pull.resize(t.x.len(), [0.0; 2]);
        touched.clear();
        for a in atts.iter_mut() {
            if !a.alive || a.best < 0 {
                continue;
            }
            if a.best_d < kill2 {
                a.alive = false;
                a.end = a.best;
                continue;
            }
            let b = a.best as usize;
            let d = a.best_d.sqrt();
            if pull[b] == [0.0, 0.0] {
                touched.push(a.best);
            }
            pull[b][0] += (a.pos[0] - t.x[b]) / d;
            pull[b][1] += (a.pos[1] - t.y[b]) / d;
        }
        fresh.clear();
        for &s in &touched {
            let v = pull[s as usize];
            pull[s as usize] = [0.0, 0.0];
            let len = v[0].hypot(v[1]);
            if len < 0.05 {
                continue;
            }
            let dir = [v[0] / len, v[1] / len];
            grew.resize(t.x.len().max(grew.len()), vec![]);
            if grew[s as usize].iter().any(|d| d[0] * dir[0] + d[1] * dir[1] > 0.9) {
                continue;
            }
            grew[s as usize].push(dir);
            let (x, y) = (t.x[s as usize] + dir[0] * step + (noise() - 0.5) * 0.8, t.y[s as usize] + dir[1] * step + (noise() - 0.5) * 0.8);
            fresh.push(t.add(x, y, s, t.theme[s as usize]));
        }
    }
    for a in atts.iter_mut() {
        if a.end < 0 {
            a.end = if a.best >= 0 { a.best } else { t.theme.iter().position(|&g| g == a.theme).unwrap_or(0) as i32 };
        }
    }
    let grown = t.x.len();
    let mut kids: Vec<Vec<u32>> = vec![vec![]; grown];
    for s in 0..grown {
        if t.parent[s] >= 0 {
            kids[t.parent[s] as usize].push(s as u32);
        }
    }
    // Suaviza os galhos: o crescimento em passos deixa quinas que parecem serrilhado.
    for _ in 0..3 {
        for s in (0..grown).rev() {
            if t.parent[s] < 0 || kids[s].len() != 1 {
                continue;
            }
            let (p, k) = (t.parent[s] as usize, kids[s][0] as usize);
            t.x[s] = (t.x[s] * 2.0 + t.x[p] + t.x[k]) / 4.0;
            t.y[s] = (t.y[s] * 2.0 + t.y[p] + t.y[k]) / 4.0;
        }
    }
    // Cada nota nova ganha um galho próprio: fica oculto no início e cresce na animação.
    let mut picked: Vec<(u32, i32, Vec<i32>)> = vec![];
    for &id in &input.fresh {
        let x = &input.nodes[id as usize];
        let g = x.theme;
        let mut r = rng(x.key.wrapping_mul(0xD6E8_FEB8_6659_FD93));
        let s = sector[g as usize];
        let a = s.0 + (0.1 + r() * 0.8) * (s.1 - s.0);
        let d = 0.55 + r() * 0.4;
        let target = [a.cos() * d * RX, CY + a.sin() * d * RY];
        let mut from = -1i32;
        let mut best = f32::MAX;
        for k in 0..grown {
            if t.theme[k] != g {
                continue;
            }
            let q = (t.x[k] - target[0]).powi(2) + (t.y[k] - target[1]).powi(2);
            if q < best {
                best = q;
                from = k as i32;
            }
        }
        if from < 0 {
            from = t.add(a.cos() * 10.0, CY + a.sin() * 10.0, -1, g);
        }
        let len = [62.0, 52.0, 46.0][tier];
        let (fx, fy) = (t.x[from as usize], t.y[from as usize]);
        let away = fx.hypot(fy - CY).max(1.0);
        let (ax, ay) = (fx / away, (fy - CY) / away);
        let bend = (r() - 0.5) * 0.9;
        let (ex, ey) = (fx + (ax * bend.cos() - ay * bend.sin()) * len, fy + (ax * bend.sin() + ay * bend.cos()) * len);
        let parts = (len / step).ceil() as usize;
        let mut chain = Vec::with_capacity(parts);
        let mut parent = from;
        for k in 1..=parts {
            let f = k as f32 / parts as f32;
            let swing = (f * std::f32::consts::PI).sin() * len * 0.1 * bend.signum();
            parent = t.add(fx + (ex - fx) * f - ay * swing, fy + (ey - fy) * f + ax * swing, parent, g);
            chain.push(parent);
        }
        picked.push((id, parent, chain));
    }
    let n = t.x.len();
    let mut kids: Vec<Vec<u32>> = vec![vec![]; n];
    for s in 0..n {
        if t.parent[s] >= 0 {
            kids[t.parent[s] as usize].push(s as u32);
        }
    }
    let mut weight = vec![0f32; n];
    for a in &atts {
        let w = if a.item >= 0 { 1.0 + input.nodes[a.item as usize].uses * 3.0 } else { 0.5 };
        let mut s = a.end;
        while s >= 0 {
            weight[s as usize] += w;
            s = t.parent[s as usize];
        }
    }
    for (_, end, _) in &picked {
        let mut s = *end;
        while s >= grown as i32 {
            weight[s as usize] += 1.0;
            s = t.parent[s as usize];
        }
    }
    // Quais trechos levam a algum item: sobe de cada ponta com item até o centro.
    let mut bears = vec![false; n];
    for a in atts.iter().filter(|a| a.item >= 0) {
        let mut s = a.end;
        while s >= 0 && !bears[s as usize] {
            bears[s as usize] = true;
            s = t.parent[s as usize];
        }
    }
    for (_, end, _) in &picked {
        let mut s = *end;
        while s >= 0 && !bears[s as usize] {
            bears[s as usize] = true;
            s = t.parent[s as usize];
        }
    }
    let mut dist = vec![0f32; n];
    let mut far = 1f32;
    for s in 0..n {
        if t.parent[s] >= 0 {
            let p = t.parent[s] as usize;
            dist[s] = dist[p] + (t.x[s] - t.x[p]).hypot(t.y[s] - t.y[p]);
        }
        far = far.max(dist[s]);
    }
    // Galho fino, com o brilho dando o corpo: tronco grosso demais escondia o que cresce dele.
    let (w0, wk) = ([0.7, 0.52, 0.4][tier], [0.4, 0.17, 0.08][tier]);
    let width = |s: usize| w0 + wk * weight[s].powf(0.45);
    let main = |s: usize| kids[s].iter().copied().max_by(|a, b| weight[*a as usize].total_cmp(&weight[*b as usize]));
    let alpha = [0.95, 0.9, 0.82][tier];
    let (mut mesh, mut dots) = (LineMesh::default(), DotList::default());
    let quad = |mesh: &mut LineMesh, s: usize| -> Option<Span> {
        let p = t.parent[s];
        if p < 0 {
            return None;
        }
        let p = p as usize;
        let d = unit(t.x[s] - t.x[p], t.y[s] - t.y[p]);
        let start = if t.parent[p] >= 0 {
            let pp = t.parent[p] as usize;
            let up = unit(t.x[p] - t.x[pp], t.y[p] - t.y[pp]);
            unit(d[0] + up[0], d[1] + up[1])
        } else {
            d
        };
        let end = match main(s) {
            Some(c) => {
                let c = c as usize;
                let on = unit(t.x[c] - t.x[s], t.y[c] - t.y[s]);
                unit(d[0] + on[0], d[1] + on[1])
            }
            None => d,
        };
        let is_main = main(p) == Some(s as u32);
        let g = t.theme[s] as f32;
        let delay = g * 0.012;
        let begin = mesh.indices.len() as u32;
        let wa = if is_main { width(p) } else { width(s) * 1.15 };
        // Ramo que não leva a item nenhum é só textura: fica atrás, mais apagado.
        let alpha = if bears[s] { alpha } else { alpha * 0.42 };
        let a = mesh.pair([t.x[p], t.y[p]], [-start[1], start[0]], dist[p] / far, &Stroke { width: wa, alpha, delay, themes: [g, g] });
        let b = mesh.pair([t.x[s], t.y[s]], [-end[1], end[0]], dist[s] / far, &Stroke { width: width(s), alpha, delay, themes: [g, g] });
        mesh.quad(a, b);
        Some(Span { start: begin, end: begin + 6, rev: false })
    };
    let mut span_of: Vec<Option<Span>> = vec![None; n];
    let first_fresh = grown;
    for (s, slot) in span_of.iter_mut().enumerate().take(first_fresh) {
        *slot = quad(&mut mesh, s);
    }
    let size = |u: f32| match tier {
        0 => 2.0 + u * 3.0,
        1 => 1.1 + u * 1.8,
        _ => 0.55 + u * 0.9,
    };
    let mut items: Vec<Item> = input.nodes.iter().map(|x| Item { theme: x.theme, size: size(x.uses), note: x.note, ..Default::default() }).collect();
    // A árvore não desenha ligações. Elas ficam guardadas e aparecem como arcos no foco.
    for &(a, b) in &input.links {
        if items[a as usize].neighbors.len() < 64 {
            items[a as usize].neighbors.push(b);
        }
        if items[b as usize].neighbors.len() < 64 {
            items[b as usize].neighbors.push(a);
        }
    }
    let dot_alpha = [1.0, 0.85, 0.7][tier];
    let path_of = |end: i32, spans: &[Option<Span>]| -> Vec<Span> {
        let mut out = vec![];
        let mut s = end;
        while s >= 0 {
            if let Some(x) = spans[s as usize] {
                out.push(x);
            }
            s = t.parent[s as usize];
        }
        out
    };
    // Itens que terminam na mesma ponta se afastam um pouco: dois pontos iguais não dão para clicar.
    let mut sharing = vec![0u32; n];
    let spread = [3.0, 2.2, 1.6][tier];
    for a in atts.iter().filter(|a| a.item >= 0) {
        let e = a.end as usize;
        let k = sharing[e];
        sharing[e] += 1;
        let (dx, dy) = if k == 0 {
            (0.0, 0.0)
        } else {
            let (turn, far_out) = (k as f32 * 2.4, spread * (k as f32).sqrt() * 2.0);
            (turn.cos() * far_out, turn.sin() * far_out)
        };
        let it = &mut items[a.item as usize];
        it.x = t.x[e] + dx;
        it.y = t.y[e] + dy;
        it.reach = dist[e] / far;
        it.path = path_of(a.end, &span_of);
        dots.add([it.x, it.y], it.size, dist[e] / far * 0.92, dot_alpha, it.theme, it.note);
    }
    scene.base_indices = mesh.indices.len() as u32;
    scene.base_dots = dots.dots.len() as u32;
    for (id, end, chain) in &picked {
        let top = *chain.first().unwrap_or(end);
        let mut own = vec![];
        for &s in chain {
            let q = quad(&mut mesh, s as usize);
            span_of[s as usize] = q;
            own.extend(q);
        }
        let stem = t.parent[top as usize];
        let e = *end as usize;
        let it = &mut items[*id as usize];
        it.x = t.x[e];
        it.y = t.y[e];
        it.reach = dist[e] / far;
        it.reach_from = if stem >= 0 { dist[stem as usize] / far } else { 0.0 };
        it.chain = own;
        it.path = if stem >= 0 { path_of(stem, &span_of) } else { vec![] };
        dots.add([it.x, it.y], it.size, 0.0, 1.0, it.theme, it.note);
        scene.created.push(Created { item: *id, index_end: mesh.indices.len() as u32, dot_end: dots.dots.len() as u32 });
    }
    for &g in &live {
        let a = (sector[g as usize].0 + sector[g as usize].1) * 0.5;
        let c = a.cos();
        if let Some(name) = input.themes.get(g as usize) {
            scene.labels.push(Label { text: name.clone(), x: c * (RX + 22.0), y: CY + a.sin() * (RY + 24.0), anchor: if c > 0.3 { 0.0 } else if c < -0.3 { 1.0 } else { 0.5 }, minor: false });
        }
    }
    // Fundo: poeira e a névoa de cada tema, no meio dos seus itens.
    scene.backdrop = dust(0x2007_5EED, 230);
    // O centro, de onde tudo cresce, ganha uma luz própria.
    scene.backdrop.push(crate::Dot { pos: [0.0, CY], size: 20.0, delay: 0.0, alpha: 0.34, theme: crate::NEUTRAL as f32, kind: 0.0 });
    for &g in &live {
        let mine: Vec<&Item> = items.iter().filter(|i| i.theme == g && (i.x != 0.0 || i.y != 0.0)).collect();
        if mine.is_empty() {
            continue;
        }
        let k = mine.len() as f32;
        let (cx, cy) = (mine.iter().map(|i| i.x).sum::<f32>() / k, mine.iter().map(|i| i.y).sum::<f32>() / k);
        let spread = (mine.iter().map(|i| (i.x - cx).powi(2) + (i.y - cy).powi(2)).sum::<f32>() / k).sqrt();
        scene.backdrop.push(haze([cx, cy], (spread * 1.5).clamp(60.0, 190.0), g, g as f32 * 0.012));
    }
    scene.vertices = mesh.vertices;
    scene.indices = mesh.indices;
    scene.dots = dots.dots;
    scene.items = items;
    scene
}
