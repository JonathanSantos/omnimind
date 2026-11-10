//! Universo: cada item vira um corpo num espaço de três dimensões.
//! Temas são aglomerados, assuntos ligados ficam perto, e as ligações viram linhas de constelação.
//! Só geometria e projeção: nada aqui toca GPU, janela ou relógio.

use crate::{rng, tier_of, Input, SLOTS};
use std::f32::consts::PI;

pub type V3 = [f32; 3];

pub fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn mul(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
pub fn len(a: V3) -> f32 {
    dot(a, a).sqrt()
}
pub fn unit(a: V3) -> V3 {
    let l = len(a);
    if l < 1e-6 { [0.0, 1.0, 0.0] } else { mul(a, 1.0 / l) }
}
pub fn lerp(a: V3, b: V3, t: f32) -> V3 {
    add(a, mul(sub(b, a), t))
}

#[derive(Clone, Debug, Default)]
pub struct Body {
    pub pos: V3,
    /// Raio do corpo com o halo, em unidades do espaço.
    pub size: f32,
    pub theme: u8,
    /// Nota é planeta, com anel. Memória é estrela.
    pub note: bool,
    pub fresh: bool,
    /// Momento em que aparece na abertura, de 0 a 1.
    pub delay: f32,
    /// Todos os vizinhos, para o foco e a leitura. As linhas da base mostram só os mais próximos.
    pub neighbors: Vec<u32>,
}

/// Aglomerado de um tema.
#[derive(Clone, Debug)]
pub struct Cluster {
    pub name: String,
    pub theme: u8,
    pub center: V3,
    pub radius: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Cosmos {
    pub bodies: Vec<Body>,
    /// Linhas de constelação da base: o esqueleto, não todas as ligações.
    pub lines: Vec<(u32, u32)>,
    pub clusters: Vec<Cluster>,
    /// Sistemas: os subtemas com nome, cada projeto em volta da sua nota.
    pub systems: Vec<Cluster>,
    /// Raio que contém tudo, a partir da origem.
    pub radius: f32,
    pub tier: usize,
    pub open_seconds: f32,
}

/// Pontos bem espalhados numa esfera unitária.
fn fibonacci(i: usize, n: usize) -> V3 {
    if n <= 1 {
        return [0.0, 0.0, 0.0];
    }
    let golden = PI * (3.0 - 5f32.sqrt());
    let y = 1.0 - 2.0 * (i as f32 + 0.5) / n as f32;
    let r = (1.0 - y * y).max(0.0).sqrt();
    let a = golden * i as f32;
    [a.cos() * r, y, a.sin() * r]
}

/// Qual tema ocupa qual lugar: temas com muitas ligações entre si ficam vizinhos.
fn seat_themes(present: &[u8], affinity: &[[f32; SLOTS]; SLOTS], seats: &[V3]) -> Vec<usize> {
    let k = present.len();
    let mut best: Vec<usize> = (0..k).collect();
    if k < 3 {
        return best;
    }
    let cost = |perm: &[usize]| {
        let mut c = 0.0;
        for i in 0..k {
            for j in i + 1..k {
                let w = affinity[present[i] as usize][present[j] as usize];
                if w > 0.0 {
                    c += w * len(sub(seats[perm[i]], seats[perm[j]]));
                }
            }
        }
        c
    };
    let mut best_cost = cost(&best);
    // Até sete temas: trocar pares até não melhorar chega ao mesmo lugar que testar tudo, e é estável.
    let mut perm = best.clone();
    let mut improved = true;
    while improved {
        improved = false;
        for i in 0..k {
            for j in i + 1..k {
                perm.swap(i, j);
                let c = cost(&perm);
                if c + 1e-4 < best_cost {
                    best_cost = c;
                    best.copy_from_slice(&perm);
                    improved = true;
                } else {
                    perm.swap(i, j);
                }
            }
        }
    }
    best
}

pub fn build(input: &Input) -> Cosmos {
    let n = input.nodes.len();
    let tier = tier_of(n);
    let mut cosmos = Cosmos { tier, open_seconds: 3.0, radius: 120.0, ..Default::default() };
    if n == 0 {
        return cosmos;
    }
    let mut count = [0usize; SLOTS];
    for x in &input.nodes {
        count[x.theme as usize] += 1;
    }
    let present: Vec<u8> = (0..SLOTS as u8).filter(|&t| count[t as usize] > 0).collect();
    let k = present.len();
    let biggest = present.iter().map(|&t| count[t as usize]).max().unwrap_or(1);
    let base = (104.0 / (biggest as f32).cbrt()).min(27.0);
    let radius_of = |t: u8| (base * (count[t as usize] as f32).cbrt()).max(22.0);
    let ring = if k == 1 { 0.0 } else { 232.0 };

    let weight = |i: usize| input.weights.get(i).copied().unwrap_or(1.0).max(0.05);
    let mut affinity = [[0f32; SLOTS]; SLOTS];
    for (i, &(a, b)) in input.links.iter().enumerate() {
        let (ta, tb) = (input.nodes[a as usize].theme as usize, input.nodes[b as usize].theme as usize);
        if ta != tb {
            affinity[ta][tb] += weight(i);
            affinity[tb][ta] += weight(i);
        }
    }
    let seats: Vec<V3> = (0..k).map(|i| mul(fibonacci(i, k), ring)).collect();
    let seat = seat_themes(&present, &affinity, &seats);
    let mut center = [[0f32; 3]; SLOTS];
    for (i, &t) in present.iter().enumerate() {
        center[t as usize] = seats[seat[i]];
        cosmos.clusters.push(Cluster { name: input.themes.get(t as usize).cloned().unwrap_or_default(), theme: t, center: seats[seat[i]], radius: radius_of(t) });
    }

    // Subtemas: grupos de itens ligados entre si ganham um canto próprio do aglomerado.
    let mut subs: Vec<Vec<u32>> = vec![Vec::new(); SLOTS];
    for x in &input.nodes {
        let list = &mut subs[x.theme as usize];
        if !list.contains(&x.sub) {
            list.push(x.sub);
        }
    }
    for list in &mut subs {
        list.sort_unstable();
    }
    let mut pos: Vec<V3> = Vec::with_capacity(n);
    for x in &input.nodes {
        let t = x.theme as usize;
        let r = radius_of(x.theme);
        let list = &subs[t];
        let at = list.iter().position(|s| *s == x.sub).unwrap_or(0);
        // Subtema com nome é um projeto: fica mais afastado dos outros, para se ler como um sistema.
        let named = input.sub_names.iter().any(|(id, _)| *id == x.sub);
        let corner = if list.len() < 2 { [0.0; 3] } else { mul(fibonacci(at, list.len()), r * if named { 0.7 } else { 0.6 }) };
        let mut rand = rng(x.key.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xD1B54A32D192ED03);
        // Ponto dentro de uma bola: a raiz cúbica mantém a densidade igual do centro à borda.
        let dir = unit([rand() * 2.0 - 1.0, rand() * 2.0 - 1.0, rand() * 2.0 - 1.0]);
        // A nota do projeto fica no meio do seu canto: o resto do projeto se arruma em volta.
        let spread = if x.hub { 0.0 } else { r * if list.len() < 2 { 0.9 } else if named { 0.3 } else { 0.42 } * rand().cbrt() };
        pos.push(add(center[t], add(corner, mul(dir, spread))));
    }

    if tier < 2 {
        relax(input, &mut pos, &center, &radius_of, &weight);
    }

    let size = |u: f32| match tier {
        0 => 7.0 + u * 8.0,
        1 => 4.6 + u * 5.5,
        _ => 3.0 + u * 3.6,
    };
    // O centro do quadro é o meio do que existe, e cada aglomerado mede o que de fato ocupa.
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for p in &pos {
        for a in 0..3 {
            lo[a] = lo[a].min(p[a]);
            hi[a] = hi[a].max(p[a]);
        }
    }
    let middle = mul(add(lo, hi), 0.5);
    pos.iter_mut().for_each(|p| *p = sub(*p, middle));
    for cl in &mut cosmos.clusters {
        let mine: Vec<V3> = input.nodes.iter().zip(&pos).filter(|(x, _)| x.theme == cl.theme).map(|(_, p)| *p).collect();
        let sum = mine.iter().fold([0.0; 3], |acc, p| add(acc, *p));
        cl.center = mul(sum, 1.0 / mine.len().max(1) as f32);
        cl.radius = mine.iter().map(|p| len(sub(*p, cl.center))).fold(0.0, f32::max) + size(0.5);
    }
    let mut reach = 0f32;
    cosmos.bodies = input
        .nodes
        .iter()
        .enumerate()
        .map(|(i, x)| {
            reach = reach.max(len(pos[i]));
            Body { pos: pos[i], size: size(x.uses) * if x.hub { 1.45 } else { 1.0 }, theme: x.theme, note: x.note, fresh: x.fresh, delay: 0.0, neighbors: Vec::new() }
        })
        .collect();
    cosmos.radius = (reach + 16.0).max(120.0);
    let mut jitter = rng(0x5EED_C05A);
    for b in &mut cosmos.bodies {
        b.delay = 0.12 + 0.5 * (len(b.pos) / cosmos.radius) + jitter() * 0.08;
    }
    for &(a, b) in &input.links {
        if cosmos.bodies[a as usize].neighbors.len() < 96 {
            cosmos.bodies[a as usize].neighbors.push(b);
        }
        if cosmos.bodies[b as usize].neighbors.len() < 96 {
            cosmos.bodies[b as usize].neighbors.push(a);
        }
    }
    for (id, name) in &input.sub_names {
        let mine: Vec<usize> = (0..n).filter(|&i| input.nodes[i].sub == *id).collect();
        // Um corpo sozinho já tem o próprio nome na ficha: o do sistema só repetiria.
        if mine.len() < 2 || name.is_empty() {
            continue;
        }
        let sum = mine.iter().fold([0.0; 3], |acc, &i| add(acc, cosmos.bodies[i].pos));
        let middle = mul(sum, 1.0 / mine.len() as f32);
        let radius = mine.iter().map(|&i| len(sub(cosmos.bodies[i].pos, middle))).fold(0.0, f32::max) + size(0.3);
        cosmos.systems.push(Cluster { name: name.clone(), theme: input.nodes[mine[0]].theme, center: middle, radius });
    }
    cosmos.lines = skeleton(input, &pos, [3, 2, 2][tier], &weight);
    cosmos
}

/// Assenta os corpos: ligação puxa, vizinho do mesmo tema empurra, o centro do tema segura.
fn relax(input: &Input, pos: &mut [V3], center: &[V3; SLOTS], radius_of: &dyn Fn(u8) -> f32, weight: &dyn Fn(usize) -> f32) {
    let n = pos.len();
    let mut by_theme: Vec<Vec<u32>> = vec![Vec::new(); SLOTS];
    for (i, x) in input.nodes.iter().enumerate() {
        by_theme[x.theme as usize].push(i as u32);
    }
    let mut degree = vec![0f32; n];
    for (i, &(a, b)) in input.links.iter().enumerate() {
        degree[a as usize] += weight(i);
        degree[b as usize] += weight(i);
    }
    // Distância típica entre vizinhos para o tema encher o próprio raio.
    let spacing: Vec<f32> = by_theme.iter().enumerate().map(|(t, list)| radius_of(t as u8) * 1.75 / (list.len().max(1) as f32).cbrt()).collect();
    let rounds = if n > 600 { 36 } else { 80 };
    let mut push = vec![[0f32; 3]; n];
    for round in 0..rounds {
        let heat = 1.0 - round as f32 / rounds as f32;
        push.iter_mut().for_each(|p| *p = [0.0; 3]);
        for (t, list) in by_theme.iter().enumerate() {
            let rest = spacing[t];
            for (i, &a) in list.iter().enumerate() {
                for &b in &list[i + 1..] {
                    let d = sub(pos[a as usize], pos[b as usize]);
                    let l = len(d).max(0.5);
                    if l > rest * 2.4 {
                        continue;
                    }
                    let f = mul(d, (rest * rest) / (l * l * l) * 1.6);
                    push[a as usize] = add(push[a as usize], f);
                    push[b as usize] = sub(push[b as usize], f);
                }
            }
        }
        for (i, &(a, b)) in input.links.iter().enumerate() {
            let (a, b) = (a as usize, b as usize);
            let same = input.nodes[a].theme == input.nodes[b].theme;
            let d = sub(pos[b], pos[a]);
            let l = len(d).max(0.5);
            // Mola com comprimento de repouso: ligados chegam perto, mas não um em cima do outro.
            let slack = spacing[input.nodes[a].theme as usize] * 0.8;
            if l <= slack {
                continue;
            }
            let stretch = 1.0 - slack / l;
            // Quem tem muitas ligações divide a força entre elas: um nó muito ligado não vira buraco negro.
            let pull = |deg: f32| stretch * weight(i) / deg.max(1.0) * if same { 0.5 } else { 0.04 };
            push[a] = add(push[a], mul(d, pull(degree[a])));
            push[b] = sub(push[b], mul(d, pull(degree[b])));
        }
        for (i, x) in input.nodes.iter().enumerate() {
            let c = center[x.theme as usize];
            let r = radius_of(x.theme);
            let step = mul(push[i], heat);
            let l = len(step);
            let step = if l > r * 0.16 { mul(step, r * 0.16 / l) } else { step };
            let mut p = add(pos[i], step);
            let off = sub(p, c);
            let d = len(off);
            if d > r {
                p = add(c, mul(off, r / d));
            }
            pos[i] = p;
        }
    }
}

/// Ligação candidata: força, comprimento e índice.
type Ranked = (f32, f32, u32);

/// Esqueleto de linhas: cada corpo fica com as ligações mais fortes e, no empate, as mais curtas.
fn skeleton(input: &Input, pos: &[V3], keep: usize, weight: &dyn Fn(usize) -> f32) -> Vec<(u32, u32)> {
    let n = pos.len();
    let mut around: Vec<Vec<Ranked>> = vec![Vec::new(); n];
    for (i, &(a, b)) in input.links.iter().enumerate() {
        let d = len(sub(pos[a as usize], pos[b as usize]));
        around[a as usize].push((weight(i), d, i as u32));
        around[b as usize].push((weight(i), d, i as u32));
    }
    let mut chosen = vec![false; input.links.len()];
    let order = |x: &Ranked, y: &Ranked| y.0.total_cmp(&x.0).then(x.1.total_cmp(&y.1)).then(x.2.cmp(&y.2));
    for list in &mut around {
        list.sort_by(order);
        for &(_, _, link) in list.iter().take(keep) {
            chosen[link as usize] = true;
        }
    }
    // Entre dois temas bastam poucas linhas: dez paralelas dizem o mesmo que três.
    let mut bridges: std::collections::BTreeMap<(u8, u8), Vec<Ranked>> = Default::default();
    for (i, &(a, b)) in input.links.iter().enumerate() {
        let (ta, tb) = (input.nodes[a as usize].theme, input.nodes[b as usize].theme);
        if chosen[i] && ta != tb {
            bridges.entry((ta.min(tb), ta.max(tb))).or_default().push((weight(i), len(sub(pos[a as usize], pos[b as usize])), i as u32));
        }
    }
    for list in bridges.values_mut() {
        list.sort_by(order);
        for &(_, _, link) in list.iter().skip(3) {
            chosen[link as usize] = false;
        }
    }
    input.links.iter().enumerate().filter(|(i, _)| chosen[*i]).map(|(_, l)| *l).collect()
}

/// Câmera em órbita da origem. Guinada e arfagem em radianos, distância em unidades do espaço.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
}

/// Abertura vertical de 40°: 1 / tan(20°).
pub const FOCAL: f32 = 2.747_477;
pub const NEAR: f32 = 4.0;

#[derive(Clone, Copy, Debug)]
pub struct Basis {
    pub right: V3,
    pub up: V3,
    pub fwd: V3,
    pub eye: V3,
}

/// Ponto na tela: posição em pontos a partir do centro (y para baixo), profundidade e pontos por unidade.
#[derive(Clone, Copy, Debug)]
pub struct Seen {
    pub x: f32,
    pub y: f32,
    pub depth: f32,
    pub scale: f32,
}

impl Camera {
    pub fn basis(&self) -> Basis {
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        let eye = mul([cp * self.yaw.sin(), sp, cp * self.yaw.cos()], self.dist);
        let fwd = unit(mul(eye, -1.0));
        let right = unit(cross(fwd, [0.0, 1.0, 0.0]));
        let up = cross(right, fwd);
        Basis { right, up, fwd, eye }
    }

    /// Distância em que uma esfera de raio `radius` cabe inteira num quadro `w` por `h`.
    pub fn fit(radius: f32, w: f32, h: f32) -> f32 {
        let aspect = (w / h.max(1.0)).min(1.0);
        radius * 0.96 * (FOCAL * FOCAL + aspect * aspect).sqrt() / aspect
    }
}

impl Basis {
    pub fn see(&self, p: V3, h: f32) -> Option<Seen> {
        let d = sub(p, self.eye);
        let depth = dot(d, self.fwd);
        if depth < NEAR {
            return None;
        }
        let scale = FOCAL / depth * h * 0.5;
        Some(Seen { x: dot(d, self.right) * scale, y: -dot(d, self.up) * scale, depth, scale })
    }
}

/// Arco entre dois corpos, curvado para fora do centro: a luz da leitura viaja por ele.
pub fn arc(a: V3, b: V3, t: f32) -> V3 {
    let mid = lerp(a, b, 0.5);
    let bow = len(sub(b, a)) * 0.32;
    let out = if len(mid) < 1.0 { unit(cross(sub(b, a), [0.3, 1.0, 0.2])) } else { unit(mid) };
    let c = add(mid, mul(out, bow));
    let u = 1.0 - t;
    add(add(mul(a, u * u), mul(c, 2.0 * u * t)), mul(b, t * t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assign_subs, Node};

    fn sample(n: usize, themes: usize) -> Input {
        let mut r = rng(7);
        let mut nodes: Vec<Node> = (0..n).map(|i| Node { theme: (i % themes) as u8, sub: 0, uses: r(), fresh: false, key: i as u64 * 131 + 1, note: i % 3 == 0, hub: false }).collect();
        let mut links = vec![];
        for i in themes..n {
            // Quase tudo liga dentro do próprio tema; uma em cada nove cruza.
            let same = (r() * (i / themes) as f32) as usize * themes + i % themes;
            links.push((same as u32, i as u32));
            if i % 9 == 0 {
                links.push(((r() * i as f32) as u32, i as u32));
            }
        }
        links.retain(|l| l.0 != l.1);
        assign_subs(&mut nodes, &links);
        Input { nodes, weights: vec![], sub_names: vec![], base_links: links.len(), links, fresh: vec![], themes: (0..themes).map(|t| format!("T{t}")).collect() }
    }

    #[test]
    fn every_body_is_finite_inside_the_radius_and_deterministic() {
        for n in [1, 2, 40, 177, 900, 4000] {
            let input = sample(n, 5.min(n));
            let c = build(&input);
            assert_eq!(c.bodies.len(), n);
            for b in &c.bodies {
                assert!(b.pos.iter().all(|v| v.is_finite()), "{:?}", b.pos);
                assert!(len(b.pos) <= c.radius, "{} fora de {}", len(b.pos), c.radius);
                assert!((0.0..=1.0).contains(&b.delay));
            }
            let again = build(&input);
            assert_eq!(c.bodies[n / 2].pos, again.bodies[n / 2].pos, "determinístico");
            assert_eq!(c.lines, again.lines);
        }
        assert!(build(&Input::default()).bodies.is_empty());
    }

    #[test]
    fn same_theme_sits_closer_than_other_themes() {
        let input = sample(177, 6);
        let c = build(&input);
        let (mut inside, mut outside, mut ni, mut no) = (0.0, 0.0, 0, 0);
        for i in 0..c.bodies.len() {
            for j in i + 1..c.bodies.len() {
                let d = len(sub(c.bodies[i].pos, c.bodies[j].pos));
                if c.bodies[i].theme == c.bodies[j].theme {
                    inside += d;
                    ni += 1;
                } else {
                    outside += d;
                    no += 1;
                }
            }
        }
        let (inside, outside) = (inside / ni as f32, outside / no as f32);
        assert!(inside * 2.0 < outside, "dentro {inside}, fora {outside}");
    }

    #[test]
    fn linked_bodies_sit_closer_than_unlinked_ones_in_the_same_theme() {
        let input = sample(240, 4);
        let c = build(&input);
        let linked: std::collections::HashSet<(u32, u32)> = input.links.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect();
        let (mut near, mut far, mut nn, mut nf) = (0.0, 0.0, 0, 0);
        for i in 0..c.bodies.len() as u32 {
            for j in i + 1..c.bodies.len() as u32 {
                if c.bodies[i as usize].theme != c.bodies[j as usize].theme {
                    continue;
                }
                let d = len(sub(c.bodies[i as usize].pos, c.bodies[j as usize].pos));
                if linked.contains(&(i, j)) {
                    near += d;
                    nn += 1;
                } else {
                    far += d;
                    nf += 1;
                }
            }
        }
        assert!(near / (nn as f32) < far / (nf as f32) * 0.9, "ligados {}, soltos {}", near / nn as f32, far / nf as f32);
    }

    #[test]
    fn skeleton_keeps_few_lines_but_every_linked_body_keeps_one() {
        // Trinta itens com a mesma tag: 435 ligações que dizem a mesma coisa.
        let mut nodes: Vec<Node> = (0..30).map(|i| Node { theme: 0, sub: 0, uses: 0.2, fresh: false, key: i + 1, note: false, hub: false }).collect();
        let mut links = vec![];
        for a in 0..30u32 {
            for b in a + 1..30 {
                links.push((a, b));
            }
        }
        assign_subs(&mut nodes, &links);
        let c = build(&Input { nodes, weights: vec![], sub_names: vec![], base_links: links.len(), links, fresh: vec![], themes: vec!["A".into()] });
        assert!(c.lines.len() <= 90, "{} linhas", c.lines.len());
        let mut touched = [false; 30];
        for &(a, b) in &c.lines {
            touched[a as usize] = true;
            touched[b as usize] = true;
        }
        assert!(touched.iter().all(|t| *t));
        assert_eq!(c.bodies[0].neighbors.len(), 29, "o foco ainda mostra todas");
    }

    #[test]
    fn a_named_subtheme_becomes_a_system_around_its_hub() {
        let mut input = sample(120, 3);
        // Vinte itens do tema 0 são o projeto "Vitrola", e o primeiro é a nota dele.
        let mine: Vec<usize> = (0..120).filter(|i| i % 3 == 0).take(20).collect();
        for &i in &mine {
            input.nodes[i].sub = 9_000;
        }
        input.nodes[mine[0]].hub = true;
        input.sub_names = vec![(9_000, "Vitrola".into()), (9_001, "Vazio".into())];
        let c = build(&input);
        assert_eq!(c.systems.len(), 1, "subtema sem corpos não vira sistema");
        let system = &c.systems[0];
        assert_eq!(system.name, "Vitrola");
        for &i in &mine {
            assert!(len(sub(c.bodies[i].pos, system.center)) <= system.radius, "corpo {i} fora do sistema");
        }
        // Os corpos do projeto ficam juntos: mais perto entre si que do resto do tema.
        let others: Vec<usize> = (0..120).filter(|i| i % 3 == 0 && !mine.contains(i)).collect();
        let spread = |set: &[usize]| set.iter().map(|&i| len(sub(c.bodies[i].pos, system.center))).sum::<f32>() / set.len() as f32;
        assert!(spread(&mine) < spread(&others), "dentro {} fora {}", spread(&mine), spread(&others));
        assert!(c.bodies[mine[0]].size > c.bodies[mine[1]].size * 1.2 || input.nodes[mine[1]].uses > input.nodes[mine[0]].uses);
    }

    #[test]
    fn camera_projects_the_origin_to_the_center_and_fits_the_radius() {
        let cam = Camera { yaw: 0.7, pitch: 0.3, dist: Camera::fit(300.0, 1200.0, 800.0) };
        let b = cam.basis();
        let o = b.see([0.0; 3], 800.0).unwrap();
        assert!(o.x.abs() < 1e-3 && o.y.abs() < 1e-3);
        assert!((o.depth - cam.dist).abs() < 1e-2);
        // Um ponto na borda da esfera, de lado para a câmera, fica dentro do quadro.
        let edge = b.see(mul(b.up, 300.0), 800.0).unwrap();
        assert!(edge.y < 0.0 && edge.y.abs() < 400.0, "{}", edge.y);
        // Atrás da câmera não existe na tela.
        assert!(b.see(add(b.eye, mul(b.fwd, -10.0)), 800.0).is_none());
    }

    #[test]
    fn arc_starts_and_ends_on_the_bodies() {
        let (a, b) = ([10.0, 0.0, 0.0], [-40.0, 30.0, 12.0]);
        assert!(len(sub(arc(a, b, 0.0), a)) < 1e-4);
        assert!(len(sub(arc(a, b, 1.0), b)) < 1e-4);
        assert!(arc([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], 0.5).iter().all(|v| v.is_finite()));
    }
}
