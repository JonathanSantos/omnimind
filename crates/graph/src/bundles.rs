//! Feixes: itens em círculo, ligações agrupadas por subtema e tema (Holten, 2006).

use crate::mesh::Stroke;
use crate::{rng, tier_of, DotList, Form, Input, Item, Label, LineMesh, Scene, Created, Span, SLOTS};
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};

const RADIUS: f32 = 258.0;
const GAP: f32 = 0.05;
const BETA: f32 = 0.9;

fn bspline(cp: &[[f32; 2]], n: usize, out: &mut Vec<[f32; 2]>) {
    out.clear();
    let m = cp.len();
    let (first, last) = (cp[0], cp[m - 1]);
    let mut ex = Vec::with_capacity(m + 4);
    ex.extend_from_slice(&[first, first]);
    for (i, p) in cp.iter().enumerate() {
        let t = i as f32 / (m - 1) as f32;
        ex.push([BETA * p[0] + (1.0 - BETA) * (first[0] + t * (last[0] - first[0])), BETA * p[1] + (1.0 - BETA) * (first[1] + t * (last[1] - first[1]))]);
    }
    ex.extend_from_slice(&[last, last]);
    let segs = ex.len() - 3;
    for k in 0..n {
        let u = k as f32 / (n - 1) as f32 * segs as f32;
        let s = (u as usize).min(segs - 1);
        let f = u - s as f32;
        let (f2, f3) = (f * f, f * f * f);
        let b = [(1.0 - 3.0 * f + 3.0 * f2 - f3) / 6.0, (4.0 - 6.0 * f2 + 3.0 * f3) / 6.0, (1.0 + 3.0 * f + 3.0 * f2 - 3.0 * f3) / 6.0, f3 / 6.0];
        let mut p = [0.0, 0.0];
        for (j, w) in b.iter().enumerate() {
            p[0] += w * ex[s + j][0];
            p[1] += w * ex[s + j][1];
        }
        out.push(p);
    }
}

fn polar(a: f32, r: f32) -> [f32; 2] {
    [a.cos() * r, a.sin() * r]
}

pub fn build(input: &Input) -> Scene {
    let n = input.nodes.len();
    let mut scene = Scene::empty(Form::Bundles);
    scene.span = 0.24;
    scene.head = 0.1;
    if n == 0 {
        return scene;
    }
    let tier = tier_of(n);
    scene.tier = tier;
    scene.open_seconds = if tier == 2 { 4.2 } else { 3.4 };
    let (mut mesh, mut dots) = (LineMesh::default(), DotList::default());
    let mut order: Vec<u32> = (0..n as u32).collect();
    order.sort_by_key(|&i| {
        let x = &input.nodes[i as usize];
        (x.theme, x.sub, x.key)
    });
    let present: std::collections::HashSet<u8> = input.nodes.iter().map(|x| x.theme).collect();
    let step = (TAU - GAP * present.len() as f32) / n as f32;
    let mut angle = vec![0f32; n];
    let mut theme_span: [Option<(f32, f32)>; SLOTS] = [None; SLOTS];
    let mut sub_span: HashMap<u32, (f32, f32)> = HashMap::new();
    let mut at = -FRAC_PI_2;
    let mut last_theme = u8::MAX;
    for &i in &order {
        let x = &input.nodes[i as usize];
        if x.theme != last_theme {
            if last_theme != u8::MAX {
                at += GAP;
            }
            theme_span[x.theme as usize] = Some((at, at));
            last_theme = x.theme;
        }
        sub_span.entry(x.sub).or_insert((at, at));
        angle[i as usize] = at + step * 0.5;
        at += step;
        sub_span.get_mut(&x.sub).unwrap().1 = at;
        theme_span[x.theme as usize].as_mut().unwrap().1 = at;
    }
    let mid = |s: (f32, f32)| (s.0 + s.1) * 0.5;
    for (g, s) in theme_span.iter().enumerate() {
        let Some(s) = *s else { continue };
        let pts: Vec<[f32; 2]> = (0..=48).map(|k| polar(s.0 + (s.1 - s.0) * k as f32 / 48.0, RADIUS + 9.0)).collect();
        mesh.ribbon(&pts, &Stroke { width: 2.2, alpha: 0.95, delay: g as f32 * 0.028, themes: [g as f32; 2] });
        let a = mid(s);
        let c = a.cos();
        if let Some(t) = input.themes.get(g) {
            scene.labels.push(Label { text: t.clone(), x: c * (RADIUS + 26.0), y: a.sin() * (RADIUS + 22.0), anchor: if c > 0.35 { 0.0 } else if c < -0.35 { 1.0 } else { 0.5 } });
        }
    }
    let size = |u: f32| match tier {
        0 => 2.0 + u * 3.6,
        1 => 1.1 + u * 2.4,
        _ => 0.7 + u * 1.5,
    };
    let mut items: Vec<Item> = input
        .nodes
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let p = polar(angle[i], RADIUS);
            Item { x: p[0], y: p[1], size: size(x.uses), theme: x.theme, note: x.note, ..Default::default() }
        })
        .collect();
    for &i in &order {
        let x = &input.nodes[i as usize];
        if !x.fresh {
            let it = &items[i as usize];
            let delay = 0.02 + (angle[i as usize] + FRAC_PI_2) / 6.3 * 0.22;
            dots.add([it.x, it.y], it.size, delay, 1.0, x.theme, x.note);
        }
    }
    // Com zoom a curva aparece de perto: poucos pontos virariam quinas.
    let samples = [44, 26, 12][tier];
    let width = [0.95, 0.7, 0.5][tier];
    let base_alpha = (6.5 / (input.links.len().max(1) as f32).sqrt()).min(0.72);
    let mut r = rng(77);
    let mut pts = Vec::with_capacity(samples);
    let anchor = |i: usize| {
        let x = &input.nodes[i];
        let (sa, ta) = (mid(sub_span[&x.sub]), mid(theme_span[x.theme as usize].unwrap()));
        (polar(sa, RADIUS * 0.72), polar(ta, RADIUS * 0.44))
    };
    let mut edge = |mesh: &mut LineMesh, items: &mut Vec<Item>, a: u32, b: u32, delay: f32, fresh: bool| {
        let (ia, ib) = (a as usize, b as usize);
        let (xa, xb) = (&input.nodes[ia], &input.nodes[ib]);
        let (pa, pb) = (anchor(ia), anchor(ib));
        let (ka, kb) = ([items[ia].x, items[ia].y], [items[ib].x, items[ib].y]);
        let cp: Vec<[f32; 2]> = if xa.theme == xb.theme && xa.sub == xb.sub {
            vec![ka, pa.0, kb]
        } else if xa.theme == xb.theme {
            vec![ka, pa.0, pa.1, pb.0, kb]
        } else {
            vec![ka, pa.0, pa.1, [(pa.1[0] + pb.1[0]) * 0.14, (pa.1[1] + pb.1[1]) * 0.14], pb.1, pb.0, kb]
        };
        bspline(&cp, samples, &mut pts);
        let alpha = if xa.theme == xb.theme { base_alpha } else { (base_alpha * 1.6).min(0.8) };
        let (start, end) = mesh.ribbon(&pts, &Stroke { width, alpha, delay, themes: [xa.theme as f32, xb.theme as f32] });
        items[ia].edges.push(Span { start, end, rev: false });
        items[ia].neighbors.push(b);
        if !fresh {
            items[ib].edges.push(Span { start, end, rev: true });
            items[ib].neighbors.push(a);
        }
    };
    for &(a, b) in &input.links[..input.base_links] {
        edge(&mut mesh, &mut items, a, b, 0.2 + r() * 0.54, false);
    }
    scene.base_indices = mesh.indices.len() as u32;
    scene.base_dots = dots.dots.len() as u32;
    for &id in &input.fresh {
        let it = &items[id as usize];
        dots.add([it.x, it.y], it.size, 0.0, 1.0, it.theme, it.note);
        for &(a, b) in &input.links[input.base_links..] {
            if a == id {
                edge(&mut mesh, &mut items, a, b, 0.0, true);
            } else if b == id {
                edge(&mut mesh, &mut items, b, a, 0.0, true);
            }
        }
        scene.created.push(Created { item: id, index_end: mesh.indices.len() as u32, dot_end: dots.dots.len() as u32 });
    }
    scene.vertices = mesh.vertices;
    scene.indices = mesh.indices;
    scene.dots = dots.dots;
    scene.items = items;
    scene
}
