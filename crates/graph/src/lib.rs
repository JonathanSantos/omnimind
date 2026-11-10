//! Geometria dos grafos Feixes e Raízes. Não toca GPU nem janela: recebe nós e ligações,
//! devolve fitas de triângulos e pontos prontos para desenhar. Roda fora do quadro.

mod bundles;
mod clusters;
pub mod cosmos;
mod mesh;
mod roots;

pub use clusters::assign_subs;
pub use mesh::{Dot, LineVertex};

use mesh::{DotList, LineMesh};

/// Fatias de cor. O tema oitavo em diante vira "Outros": cor nunca se repete.
pub const SLOTS: usize = 7;
/// Cor neutra, logo depois das fatias de tema: é a da poeira de fundo.
pub const NEUTRAL: u8 = SLOTS as u8;
const FRAME_W: f32 = 900.0;
const FRAME_H: f32 = 600.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Bundles,
    Roots,
    /// Três dimensões: a geometria sai de `cosmos`, não de `Scene`.
    Universe,
}

#[derive(Clone, Copy, Debug)]
pub struct Node {
    /// Fatia de tema, de 0 a 6.
    pub theme: u8,
    pub sub: u32,
    /// Uso relativo, de 0 a 1.
    pub uses: f32,
    /// Ainda não existe: nasce durante a animação de criação.
    pub fresh: bool,
    /// Identidade estável entre reconstruções: o mesmo nó fica no mesmo lugar.
    pub key: u64,
    /// Nota, e não memória: anel em Feixes e Raízes, planeta no Universo.
    pub note: bool,
    /// Nota que declara um projeto: é o centro do seu subtema.
    pub hub: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Input {
    pub nodes: Vec<Node>,
    /// Ligações antigas primeiro; as de `base_links` em diante têm uma ponta nova.
    pub links: Vec<(u32, u32)>,
    /// Força de cada ligação, na mesma ordem. Vazio vale 1 para todas.
    pub weights: Vec<f32>,
    pub base_links: usize,
    /// Nós novos, na ordem em que nascem.
    pub fresh: Vec<u32>,
    /// Nome de cada fatia.
    pub themes: Vec<String>,
    /// Nome dos subtemas que têm nome: os projetos, quando a cor é o grupo.
    pub sub_names: Vec<(u32, String)>,
}

/// Trecho do índice de triângulos. `rev` inverte o sentido do desenho.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub rev: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Item {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub theme: u8,
    /// Nota, e não memória: o ponto vira anel.
    pub note: bool,
    pub edges: Vec<Span>,
    pub neighbors: Vec<u32>,
    /// Caminho do centro até a ponta, em Raízes.
    pub path: Vec<Span>,
    pub reach: f32,
    /// Galho próprio de uma nota nova, em Raízes.
    pub chain: Vec<Span>,
    pub reach_from: f32,
}

#[derive(Clone, Debug)]
pub struct Label {
    pub text: String,
    pub x: f32,
    pub y: f32,
    /// Âncora horizontal: 0 à esquerda, 0,5 no centro, 1 à direita.
    pub anchor: f32,
    /// Nome de subtema, como um projeto dentro do grupo: menor e mais apagado.
    pub minor: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Created {
    pub item: u32,
    pub index_end: u32,
    pub dot_end: u32,
}

/// Trecho aceso por cima da base.
#[derive(Clone, Copy, Debug)]
pub struct Part {
    pub offset: u32,
    pub count: u32,
    pub rev: bool,
    pub from: f32,
    pub to: f32,
    pub start: f32,
    pub duration: f32,
    pub hi: f32,
    pub widen: f32,
    pub rest: f32,
    pub fade: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Glow {
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub theme: u8,
    pub start: f32,
}

/// Índices e horários de tudo o que acende. Montado sob demanda.
#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub indices: Vec<u32>,
    pub parts: Vec<Part>,
    pub glows: Vec<Glow>,
}

pub struct PartSpec {
    pub start: f32,
    pub duration: f32,
    pub from: f32,
    pub to: f32,
    pub rest: f32,
    pub fade: f32,
    pub widen: f32,
}

impl Default for PartSpec {
    fn default() -> Self {
        Self { start: 0.0, duration: 0.8, from: 0.0, to: 1.0, rest: 0.3, fade: 1.6, widen: 0.8 }
    }
}

impl Plan {
    pub fn part(&mut self, scene: &Scene, spans: &[Span], rev: bool, spec: PartSpec) {
        if spans.is_empty() {
            return;
        }
        let offset = self.indices.len() as u32;
        for s in spans {
            self.indices.extend_from_slice(&scene.indices[s.start as usize..s.end as usize]);
        }
        let count = self.indices.len() as u32 - offset;
        self.parts.push(Part { offset, count, rev, from: spec.from, to: spec.to, start: spec.start, duration: spec.duration, hi: 1.0, widen: spec.widen, rest: spec.rest, fade: spec.fade });
    }

    pub fn glow(&mut self, scene: &Scene, item: u32, start: f32, scale: f32) {
        let it = &scene.items[item as usize];
        let base = [11.0, 9.0, 7.0][scene.tier];
        self.glows.push(Glow { x: it.x, y: it.y, size: base * scale, theme: it.theme, start });
    }
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub form: Form,
    pub vertices: Vec<LineVertex>,
    pub indices: Vec<u32>,
    pub dots: Vec<Dot>,
    /// O que fica atrás do desenho: poeira e a névoa de cada tema. Não é item, não se clica.
    pub backdrop: Vec<Dot>,
    pub items: Vec<Item>,
    pub labels: Vec<Label>,
    pub base_indices: u32,
    pub base_dots: u32,
    pub created: Vec<Created>,
    pub span: f32,
    pub head: f32,
    pub open_seconds: f32,
    pub tier: usize,
}

/// Arcos soltos entre pares de itens, fora da malha da base.
/// É por eles que a luz da leitura viaja de um item ao seguinte, e que Raízes mostra ligações no foco.
#[derive(Clone, Debug, Default)]
pub struct Arcs {
    pub vertices: Vec<LineVertex>,
    pub indices: Vec<u32>,
    /// Trecho de índices de cada arco, na ordem em que foram pedidos.
    pub spans: Vec<(u32, u32)>,
}

const ARC_STEPS: usize = 36;

/// Poeira de fundo: pontos miúdos espalhados pelo quadro, sempre os mesmos.
/// Dá ao desenho um espaço para existir, como o céu atrás do Universo.
pub(crate) fn dust(seed: u64, count: usize) -> Vec<Dot> {
    let mut r = rng(seed);
    (0..count)
        .map(|_| {
            let pos = [(r() * 2.0 - 1.0) * FRAME_W * 0.56, (r() * 2.0 - 1.0) * FRAME_H * 0.56];
            let faint = r();
            Dot { pos, size: 0.5 + faint * faint * 1.1, delay: 0.0, alpha: 0.18 + r() * 0.42, theme: NEUTRAL as f32, kind: 0.0 }
        })
        .collect()
}

/// Névoa de um tema: uma mancha larga e suave na cor dele.
pub(crate) fn haze(pos: [f32; 2], size: f32, theme: u8, delay: f32) -> Dot {
    Dot { pos, size, delay, alpha: 0.11, theme: theme as f32, kind: 0.0 }
}

/// Faixa de volume: decide espessuras, amostragem e passo do crescimento.
pub fn tier_of(n: usize) -> usize {
    match n {
        0..=199 => 0,
        200..=1499 => 1,
        _ => 2,
    }
}

pub fn rng(seed: u64) -> impl FnMut() -> f32 {
    let mut s = seed | 1;
    move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        ((s >> 40) as f32) / (1u64 << 24) as f32
    }
}

pub fn build(form: Form, input: &Input) -> Scene {
    match form {
        Form::Bundles => bundles::build(input),
        Form::Roots => roots::build(input),
        Form::Universe => Scene::empty(Form::Universe),
    }
}

impl Scene {
    fn empty(form: Form) -> Self {
        Scene { form, vertices: vec![], indices: vec![], dots: vec![], backdrop: vec![], items: vec![], labels: vec![], base_indices: 0, base_dots: 0, created: vec![], span: 1.0, head: 0.1, open_seconds: 3.4, tier: 0 }
    }

    /// Escala para caber no quadro de desenho, em unidades da tela por unidade do grafo.
    pub fn fit(w: f32, h: f32) -> f32 {
        (w / FRAME_W).min(h / FRAME_H)
    }

    /// Ponto do arco entre dois itens, de 0 a 1. A curva cede para o centro, como as ligações de Feixes.
    pub fn arc_at(&self, from: u32, to: u32, t: f32) -> [f32; 2] {
        let (a, b) = (&self.items[from as usize], &self.items[to as usize]);
        let pull = if self.form == Form::Roots { 0.62 } else { 0.28 };
        let c = [(a.x + b.x) * 0.5 * pull, (a.y + b.y) * 0.5 * pull];
        let u = 1.0 - t;
        [u * u * a.x + 2.0 * u * t * c[0] + t * t * b.x, u * u * a.y + 2.0 * u * t * c[1] + t * t * b.y]
    }

    /// Acrescenta o arco entre dois itens, com a cor indo do tema de um ao do outro.
    pub fn arc(&self, from: u32, to: u32, width: f32, arcs: &mut Arcs) {
        let pts: Vec<[f32; 2]> = (0..=ARC_STEPS).map(|k| self.arc_at(from, to, k as f32 / ARC_STEPS as f32)).collect();
        let themes = [self.items[from as usize].theme as f32, self.items[to as usize].theme as f32];
        let mut mesh = LineMesh { vertices: std::mem::take(&mut arcs.vertices), indices: std::mem::take(&mut arcs.indices) };
        let span = mesh.ribbon(&pts, &mesh::Stroke { width, alpha: 1.0, delay: 0.0, themes });
        arcs.vertices = mesh.vertices;
        arcs.indices = mesh.indices;
        arcs.spans.push(span);
    }

    /// Acende o que um item alcança. Vale para leitura ao vivo e para o foco do mouse.
    pub fn reach(&self, item: u32, start: f32, duration: f32, plan: &mut Plan) {
        let it = &self.items[item as usize];
        match self.form {
            Form::Bundles => {
                let (fw, bw): (Vec<Span>, Vec<Span>) = it.edges.iter().partition(|e| !e.rev);
                plan.part(self, &fw, false, PartSpec { start, duration, ..Default::default() });
                plan.part(self, &bw, true, PartSpec { start, duration, ..Default::default() });
                plan.glow(self, item, start, 1.6);
                for &n in &it.neighbors {
                    plan.glow(self, n, start + duration * 0.8, 1.0);
                }
            }
            Form::Roots => {
                plan.part(self, &it.path, false, PartSpec { start, duration, to: it.reach, widen: 1.1, ..Default::default() });
                plan.glow(self, item, start + duration * 0.85, 1.8);
            }
            Form::Universe => {}
        }
    }

    /// Nascimento de um item novo. Devolve quanto tempo leva até virar base.
    pub fn grow(&self, item: u32, start: f32, plan: &mut Plan) -> f32 {
        let it = &self.items[item as usize];
        match self.form {
            Form::Bundles => {
                plan.glow(self, item, start, 2.2);
                let fw: Vec<Span> = it.edges.iter().filter(|e| !e.rev).copied().collect();
                plan.part(self, &fw, false, PartSpec { start: start + 0.12, duration: 0.9, rest: 0.0, fade: 0.9, ..Default::default() });
                for &n in &it.neighbors {
                    plan.glow(self, n, start + 0.9, 1.0);
                }
                1.9
            }
            Form::Roots => {
                plan.part(self, &it.path, false, PartSpec { start, duration: 0.5, to: it.reach_from, widen: 0.9, rest: 0.0, fade: 0.9, ..Default::default() });
                plan.part(self, &it.chain, false, PartSpec { start: start + 0.42, duration: 0.8, from: it.reach_from, to: it.reach, widen: 0.5, rest: 0.0, fade: 0.8 });
                plan.glow(self, item, start + 1.15, 2.0);
                2.2
            }
            Form::Universe => 0.0,
        }
    }

    /// Item mais próximo de um ponto em unidades do grafo, ignorando quem ainda não nasceu.
    pub fn nearest(&self, x: f32, y: f32, limit: f32, hidden: &[bool]) -> Option<u32> {
        let mut best = limit * limit;
        let mut found = None;
        for (i, it) in self.items.iter().enumerate() {
            if hidden.get(i).copied().unwrap_or(false) {
                continue;
            }
            let d = (it.x - x).powi(2) + (it.y - y).powi(2);
            if d < best {
                best = d;
                found = Some(i as u32);
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(n: usize, fresh: usize) -> Input {
        let mut r = rng(11);
        let total = n + fresh;
        let mut nodes: Vec<Node> = (0..total).map(|i| Node { theme: (i % 5) as u8, sub: 0, uses: r(), fresh: i >= n, key: i as u64 * 977 + 1, note: false, hub: false }).collect();
        let mut links = vec![];
        for i in 1..n {
            links.push(((r() * i as f32) as u32, i as u32));
        }
        let base_links = links.len();
        for i in n..total {
            links.push(((i - n) as u32, i as u32));
        }
        assign_subs(&mut nodes, &links);
        Input { nodes, links, weights: vec![], sub_names: vec![], base_links, fresh: (n as u32..total as u32).collect(), themes: ["A", "B", "C", "D", "E"].map(String::from).to_vec() }
    }

    fn check(form: Form, n: usize, fresh: usize) {
        let input = sample(n, fresh);
        let s = build(form, &input);
        assert_eq!(s.items.len(), n + fresh);
        assert_eq!(s.created.len(), fresh);
        for it in &s.items {
            assert!(it.x.is_finite() && it.y.is_finite());
            assert!(it.x.abs() <= 450.0 && it.y.abs() <= 300.0, "{} {}", it.x, it.y);
        }
        assert!(s.base_indices as usize <= s.indices.len());
        assert!(s.indices.iter().all(|&i| (i as usize) < s.vertices.len()));
        assert_eq!(s.indices.len() % 3, 0);
        // Base nunca aponta para geometria de nota que ainda não nasceu.
        let mut end = s.base_indices;
        for c in &s.created {
            assert!(c.index_end >= end);
            end = c.index_end;
        }
        let again = build(form, &input);
        assert_eq!(s.vertices.len(), again.vertices.len(), "determinístico");
        assert_eq!(s.items[0].x, again.items[0].x);
    }

    #[test]
    fn bundles_scale_from_small_to_large() {
        check(Form::Bundles, 40, 4);
        check(Form::Bundles, 540, 6);
        check(Form::Bundles, 4000, 40);
    }

    #[test]
    fn roots_scale_from_small_to_large() {
        check(Form::Roots, 40, 4);
        check(Form::Roots, 540, 6);
        check(Form::Roots, 4000, 40);
    }

    #[test]
    fn empty_and_tiny_graphs_do_not_panic() {
        for form in [Form::Bundles, Form::Roots] {
            assert!(build(form, &Input::default()).items.is_empty());
            check(form, 1, 0);
            check(form, 2, 1);
        }
    }

    #[test]
    fn same_key_stays_in_place_when_others_arrive() {
        let a = sample(300, 0);
        let mut b = a.clone();
        b.nodes.push(Node { theme: 0, sub: 0, uses: 0.1, fresh: false, key: 999_999, note: false, hub: false });
        let (x, y) = (build(Form::Roots, &a), build(Form::Roots, &b));
        let moved = (x.items[10].x - y.items[10].x).hypot(x.items[10].y - y.items[10].y);
        assert!(moved < 40.0, "andou {moved}");
    }

    #[test]
    fn fresh_edges_stay_out_of_older_items() {
        let s = build(Form::Bundles, &sample(50, 5));
        let older_edges: usize = s.items[..50].iter().map(|i| i.edges.len()).sum();
        assert_eq!(older_edges, 49 * 2);
        assert!(s.items[50].edges.iter().all(|e| !e.rev));
    }

    #[test]
    fn roots_never_stack_the_same_branch() {
        let s = build(Form::Roots, &sample(540, 0));
        let mut seen = std::collections::HashMap::new();
        for q in s.vertices.chunks(4) {
            let key = ((q[2].pos[0] * 4.0) as i32, (q[2].pos[1] * 4.0) as i32);
            *seen.entry(key).or_insert(0) += 1;
        }
        assert!(seen.values().all(|n| *n <= 6), "pior {:?}", seen.values().max());
    }

    #[test]
    fn every_root_item_can_be_reached() {
        let s = build(Form::Roots, &sample(540, 0));
        let lost = (0..s.items.len() as u32).filter(|&i| s.nearest(s.items[i as usize].x, s.items[i as usize].y, 0.5, &[]) != Some(i)).count();
        assert!(lost < 8, "{lost} itens sem clique");
    }

    #[test]
    fn notes_are_rings_and_memories_are_discs_in_both_forms() {
        for form in [Form::Bundles, Form::Roots] {
            let mut input = sample(60, 3);
            for (i, n) in input.nodes.iter_mut().enumerate() {
                n.note = i % 4 == 0;
            }
            let s = build(form, &input);
            let rings = s.dots.iter().filter(|d| d.kind > 0.5).count();
            assert_eq!(rings, input.nodes.iter().filter(|n| n.note).count(), "{form:?}");
            assert!(s.items.iter().zip(&input.nodes).all(|(it, n)| it.note == n.note));
        }
    }

    #[test]
    fn roots_keep_the_links_for_the_focus_even_without_drawing_them() {
        let input = sample(80, 0);
        let s = build(Form::Roots, &input);
        let known: usize = s.items.iter().map(|i| i.neighbors.len()).sum();
        assert_eq!(known, input.links.len() * 2);
    }

    #[test]
    fn arc_runs_from_one_item_to_the_other_and_appends() {
        for form in [Form::Bundles, Form::Roots] {
            let s = build(form, &sample(40, 0));
            let (a, b) = (&s.items[3], &s.items[27]);
            let (p0, p1) = (s.arc_at(3, 27, 0.0), s.arc_at(3, 27, 1.0));
            assert!((p0[0] - a.x).abs() < 1e-3 && (p0[1] - a.y).abs() < 1e-3);
            assert!((p1[0] - b.x).abs() < 1e-3 && (p1[1] - b.y).abs() < 1e-3);
            let mut arcs = Arcs::default();
            s.arc(3, 27, 2.0, &mut arcs);
            s.arc(5, 9, 2.0, &mut arcs);
            assert_eq!(arcs.spans.len(), 2);
            assert_eq!(arcs.spans[0].1, arcs.spans[1].0, "o segundo arco começa onde o primeiro acaba");
            assert_eq!(arcs.spans[1].1 as usize, arcs.indices.len());
            assert!(arcs.indices.iter().all(|&i| (i as usize) < arcs.vertices.len()));
            assert_eq!(arcs.vertices[0].along, 0.0);
            assert_eq!(arcs.vertices.last().unwrap().along, 1.0);
        }
    }

    #[test]
    fn bundles_draw_a_skeleton_and_keep_every_link_for_the_focus() {
        // Trinta itens com a mesma tag: 435 ligações que dizem a mesma coisa.
        let mut nodes: Vec<Node> = (0..30).map(|i| Node { theme: 0, sub: 0, uses: 0.3, fresh: false, key: i + 1, note: false, hub: false }).collect();
        let mut links = vec![];
        for a in 0..30u32 {
            for b in a + 1..30 {
                links.push((a, b));
            }
        }
        assign_subs(&mut nodes, &links);
        let input = Input { nodes, weights: vec![], sub_names: vec![], base_links: links.len(), links, fresh: vec![], themes: vec!["A".into()] };
        let s = build(Form::Bundles, &input);
        // Cada ligação aparece duas vezes nos itens, uma em cada ponta: o foco continua vendo todas.
        let known: usize = s.items.iter().map(|i| i.edges.len()).sum();
        assert_eq!(known, 435 * 2);
        assert!(s.items.iter().all(|i| i.neighbors.len() == 29));
        // A base desenha bem menos que isso, e cada item mantém ao menos uma ligação nela.
        let base = s.base_indices;
        let in_base = |i: &Item| i.edges.iter().filter(|e| e.end <= base).count();
        let drawn: usize = s.items.iter().map(in_base).sum::<usize>() / 2;
        assert!(drawn <= 90, "{drawn} ligações na base");
        assert!(s.items.iter().all(|i| in_base(i) >= 1));
        assert!((base as usize) < s.indices.len(), "o resto da geometria fica depois da base");
    }

    #[test]
    fn both_forms_have_a_backdrop_that_is_not_an_item() {
        for form in [Form::Bundles, Form::Roots] {
            let s = build(form, &sample(60, 0));
            assert_eq!(s.items.len(), 60, "{form:?}: o fundo não vira item");
            let dust = s.backdrop.iter().filter(|d| d.theme as u8 == NEUTRAL && d.size < 3.0).count();
            let haze = s.backdrop.iter().filter(|d| (d.theme as u8) < NEUTRAL).count();
            assert!(dust > 100, "{form:?}: {dust} grãos de poeira");
            assert!(haze >= 5, "{form:?}: uma névoa por tema, {haze}");
            assert!(s.backdrop.iter().all(|d| d.pos[0].is_finite() && d.pos[1].is_finite()));
            assert_eq!(s.backdrop.len(), build(form, &sample(60, 0)).backdrop.len(), "determinístico");
        }
        assert!(build(Form::Bundles, &Input::default()).backdrop.is_empty());
    }

    #[test]
    fn nearest_skips_hidden_items() {
        let s = build(Form::Bundles, &sample(20, 0));
        let at = &s.items[3];
        assert_eq!(s.nearest(at.x, at.y, 8.0, &[]), Some(3));
        let mut hidden = vec![false; 20];
        hidden[3] = true;
        assert_ne!(s.nearest(at.x, at.y, 8.0, &hidden), Some(3));
    }
}
