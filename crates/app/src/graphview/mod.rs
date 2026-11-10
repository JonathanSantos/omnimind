//! Tela do grafo: guarda o que está desenhado, anima abertura, leitura e criação, e responde ao mouse.
//! A geometria sai do crate `graph`, numa thread própria: o quadro nunca espera por ela.

mod gpu;
mod universe;
mod universe_gpu;

use crate::model::{Graph, GraphNode, NodeKind};
use crate::theme::{is_dark, mono, ui as ui_style, THEME};
use crate::ui::Ui;
use gfx::wgpu::{CommandEncoder, TextureView};
use gfx::{Color, Gpu, Rect};
use gpu::{Call, DrawU, FrameU, Pipes, Target, MAX_DRAWS};
use graph::cosmos::Cosmos;
use graph::{assign_subs, Arcs, Dot, Form, Input, Node, Plan, Scene, SLOTS};
use universe::Universe;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

const FOCUS_SECONDS: f32 = 0.3;
const PULSE_SECONDS: f32 = 0.85;
const PULSE_LIFE: f32 = 3.0;
const MAX_PULSES: usize = 24;
/// Tempo que a luz leva de um item lido ao seguinte.
const TRAVEL: f32 = 0.8;
/// Duas leituras dentro desse intervalo fazem parte do mesmo percurso.
const STREAK: f32 = 25.0;
const MAX_ZOOM: f32 = 8.0;
/// Brilho das linhas: quanto a fita alarga, em pontos, e a opacidade por faixa de volume.
const GLOW_WIDTH: f32 = 6.0;
const GLOW_ALPHA: [f32; 3] = [0.12, 0.07, 0.0];
/// Halo de cada item: quantas vezes o tamanho do miolo, e a força.
const HALO_GROW: f32 = 1.9;
const HALO_ALPHA: f32 = 0.38;
/// Quantas ligações do item em foco Raízes desenha.
const FOCUS_ARCS: usize = 32;
/// Acima disso, muita coisa chegou de uma vez: o desenho só atualiza.
const MAX_BIRTHS: usize = 48;

fn smooth(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    v * v * (3.0 - 2.0 * v)
}

fn ease_out(v: f32) -> f32 {
    1.0 - (1.0 - v).powi(3)
}

fn key_of(id: &str) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for b in id.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    h | 1
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    All,
    Notes,
    Memories,
}

/// O que dá a cor no grafo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorBy {
    /// O grupo, a categoria acima do projeto. Dentro dele, cada projeto fica junto.
    Group,
    /// O projeto. Os maiores ganham cor, o resto vira "Outros".
    Project,
}

/// Subtemas de projeto não colidem com os que vêm das ligações.
const PROJECT_SUB: u32 = 1 << 20;

impl Filter {
    fn keeps(self, kind: NodeKind) -> bool {
        match self {
            Filter::All => true,
            Filter::Notes => matches!(kind, NodeKind::Note),
            Filter::Memories => !matches!(kind, NodeKind::Note),
        }
    }
}

struct Built {
    ticket: u64,
    scene: Scene,
    /// Só no Universo: a geometria em três dimensões, e quem nasce agora.
    cosmos: Option<Cosmos>,
    fresh: Vec<u32>,
    nodes: Vec<u32>,
    themes: Vec<String>,
    links: usize,
    graph: Arc<Graph>,
}

struct Birth {
    start: f32,
    ready: f32,
    item: u32,
    index_end: u32,
    dot_end: u32,
}

#[derive(Clone, Copy)]
struct Pulse {
    item: u32,
    /// Quando a leitura chega ao item. Com `from`, a luz sai `TRAVEL` segundos antes.
    start: f32,
    from: Option<u32>,
}

/// Um arco solto e o que o anima.
#[derive(Clone, Copy)]
enum ArcPart {
    Read { span: (u32, u32), start: f32, from: u32, to: u32 },
    Focus { span: (u32, u32), to: u32 },
}

struct Drag {
    last: (f32, f32),
    travelled: f32,
}

pub struct GraphView {
    pub form: Form,
    pub filter: Filter,
    pub color_by: ColorBy,
    graph: Arc<Graph>,
    /// Fora da tela não há por que montar geometria: fica marcado e monta ao voltar.
    pub visible: bool,
    known: FxHashSet<String>,
    /// O grafo que a cena desenhada mostra, que pode ser anterior a `graph` por um instante.
    drawn: Arc<Graph>,
    /// Item desenhado → nó de `drawn`.
    nodes: Vec<u32>,
    items: FxHashMap<String, u32>,
    scene: Option<Arc<Scene>>,
    pipes: Option<Pipes>,
    target: Option<Target>,
    wanted: (u32, u32),
    uploaded: bool,
    ticket: u64,
    pending: bool,
    built_tx: Sender<Built>,
    built_rx: Receiver<Built>,
    pub clock: f32,
    open_at: Option<f32>,
    want_open: bool,
    births: Vec<Birth>,
    settled_births: bool,
    pulses: Vec<Pulse>,
    fx: Plan,
    fx_dirty: bool,
    focus: Plan,
    focus_item: Option<u32>,
    focus_amt: f32,
    focus_at: f32,
    reading: f32,
    pub hover: Option<u32>,
    pub selected: Option<u32>,
    pub theme_names: Vec<String>,
    links: usize,
    dirty: bool,
    last_mouse: (f32, f32),
    pub reduce_motion: bool,
    dark_seen: bool,
    labels_alpha: f32,
    universe: Universe,
    /// Aproximação e deslocamento de Feixes e Raízes, em pontos de tela.
    zoom: f32,
    pan: (f32, f32),
    drag: Option<Drag>,
    stage: (f32, f32),
    last_read: Option<(u32, f32)>,
    focus_shown: Option<u32>,
    arcs: Arcs,
    arc_parts: Vec<ArcPart>,
    arcs_dirty: bool,
}

impl GraphView {
    pub fn new() -> Self {
        let (built_tx, built_rx) = channel();
        Self {
            form: Form::Bundles,
            filter: Filter::All,
            color_by: ColorBy::Group,
            graph: Arc::new(Graph::default()),
            visible: true,
            known: FxHashSet::default(),
            drawn: Arc::new(Graph::default()),
            nodes: vec![],
            items: FxHashMap::default(),
            scene: None,
            pipes: None,
            target: None,
            wanted: (0, 0),
            uploaded: false,
            ticket: 0,
            pending: false,
            built_tx,
            built_rx,
            clock: 0.0,
            open_at: None,
            want_open: true,
            births: vec![],
            settled_births: true,
            pulses: vec![],
            fx: Plan::default(),
            fx_dirty: false,
            focus: Plan::default(),
            focus_item: None,
            focus_amt: 0.0,
            focus_at: 0.0,
            reading: 0.0,
            hover: None,
            selected: None,
            theme_names: vec![],
            links: 0,
            dirty: true,
            last_mouse: (-1.0, -1.0),
            reduce_motion: false,
            dark_seen: false,
            labels_alpha: 0.0,
            universe: Universe::new(),
            zoom: 1.0,
            pan: (0.0, 0.0),
            drag: None,
            stage: (1.0, 1.0),
            last_read: None,
            focus_shown: None,
            arcs: Arcs::default(),
            arc_parts: vec![],
            arcs_dirty: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.graph.nodes.is_empty()
    }

    /// Grafo novo do cofre. Quem ainda não existia nasce animado.
    pub fn set_graph(&mut self, g: Arc<Graph>) {
        let first = self.known.is_empty();
        let fresh: Vec<u32> = if first {
            vec![]
        } else {
            g.nodes.iter().enumerate().filter(|(_, n)| !self.known.contains(&n.id)).map(|(i, _)| i as u32).collect()
        };
        self.known = g.nodes.iter().map(|n| n.id.clone()).collect();
        self.graph = g;
        if first {
            self.want_open = true;
        }
        if !self.visible {
            return;
        }
        let fresh = if fresh.len() > MAX_BIRTHS { vec![] } else { fresh };
        self.rebuild(fresh);
    }

    /// Volta ao começo, como quem chega na tela.
    pub fn replay(&mut self) {
        self.want_open = true;
        self.zoom = 1.0;
        self.pan = (0.0, 0.0);
        self.rebuild(vec![]);
    }

    pub fn set_form(&mut self, form: Form) {
        if self.form != form {
            self.form = form;
            self.replay();
        }
    }

    pub fn set_color_by(&mut self, by: ColorBy) {
        if self.color_by != by {
            self.color_by = by;
            self.replay();
        }
    }

    pub fn set_filter(&mut self, filter: Filter) {
        if self.filter != filter {
            self.filter = filter;
            self.replay();
        }
    }

    /// A IA acabou de ler este nó: o caminho até ele acende.
    pub fn live_read(&mut self, node: &str) {
        let Some(&item) = self.items.get(node) else { return };
        if self.universe.active() {
            self.universe.live_read(item);
            return;
        }
        if self.scene.is_none() {
            return;
        }
        self.pulses.retain(|p| self.clock - p.start < PULSE_LIFE);
        if self.pulses.len() >= MAX_PULSES {
            self.pulses.remove(0);
        }
        // Leitura em sequência: a luz viaja do item anterior até este, e só então o caminho acende.
        let from = self.last_read.filter(|(i, at)| *i != item && self.clock - at < STREAK).map(|(i, _)| i);
        let start = self.clock + if from.is_some() { TRAVEL } else { 0.0 };
        self.pulses.push(Pulse { item, start, from });
        self.last_read = Some((item, self.clock));
        self.fx_dirty = true;
        self.dirty = true;
        self.rebuild_arcs();
    }

    /// Quantas leituras da IA estão acesas agora, em qualquer das três formas.
    pub fn reads_shown(&self) -> usize {
        if self.universe.active() { self.universe.reads_shown() } else { self.pulses.iter().filter(|p| self.clock - p.start < PULSE_LIFE).count() }
    }

    /// Quantos arcos soltos existem agora: percursos de leitura e ligações do foco.
    pub fn arcs_shown(&self) -> usize {
        self.arc_parts.len()
    }

    /// Aproximação e deslocamento atuais de Feixes e Raízes.
    pub fn view(&self) -> (f32, (f32, f32)) {
        (self.zoom, self.pan)
    }

    /// Escala e origem do desenho na tela: tudo o que posiciona item, nome ou ficha passa por aqui.
    fn frame_of(&self, r: Rect) -> (f32, f32, f32) {
        (Scene::fit(r.w, r.h) * self.zoom, r.x + r.w * 0.5 + self.pan.0, r.y + r.h * 0.5 + self.pan.1)
    }

    /// Refaz os arcos soltos: um por leitura em percurso e, em Raízes, as ligações do item em foco.
    fn rebuild_arcs(&mut self) {
        self.arcs = Arcs::default();
        self.arc_parts.clear();
        self.arcs_dirty = true;
        let Some(s) = self.scene.clone() else { return };
        let n = s.items.len() as u32;
        for p in &self.pulses {
            if let Some(from) = p.from.filter(|f| *f < n && p.item < n) {
                s.arc(from, p.item, 1.6, &mut self.arcs);
                self.arc_parts.push(ArcPart::Read { span: *self.arcs.spans.last().unwrap(), start: p.start, from, to: p.item });
            }
        }
        if let (Form::Roots, Some(i)) = (s.form, self.focus_shown.filter(|i| *i < n)) {
            let hidden = self.hidden();
            for &to in s.items[i as usize].neighbors.iter().filter(|t| !hidden.get(**t as usize).copied().unwrap_or(false)).take(FOCUS_ARCS) {
                s.arc(i, to, 1.1, &mut self.arcs);
                self.arc_parts.push(ArcPart::Focus { span: *self.arcs.spans.last().unwrap(), to });
            }
        }
    }

    pub fn counts(&self) -> (usize, usize) {
        (self.nodes.len(), self.links)
    }

    /// Posição de um item na tela, dado o retângulo do grafo.
    pub fn screen_of(&self, item: u32, r: Rect) -> Option<(f32, f32)> {
        if self.universe.active() {
            return self.universe.screen_of(item, r);
        }
        let it = self.scene.as_ref()?.items.get(item as usize)?;
        let (k, cx, cy) = self.frame_of(r);
        Some((cx + it.x * k, cy + it.y * k))
    }

    /// Câmera do Universo, quando é ele que está na tela.
    pub fn universe_camera(&self) -> Option<graph::cosmos::Camera> {
        self.universe.active().then_some(self.universe.cam)
    }

    pub fn item_of(&self, node: &str) -> Option<u32> {
        self.items.get(node).copied()
    }

    pub fn node_of(&self, item: u32) -> Option<&GraphNode> {
        self.drawn.nodes.get(*self.nodes.get(item as usize)? as usize)
    }

    fn rebuild(&mut self, fresh_nodes: Vec<u32>) {
        let g = self.graph.clone();
        let filter = self.filter;
        let form = self.form;
        let color_by = self.color_by;
        self.ticket += 1;
        self.pending = true;
        let ticket = self.ticket;
        let tx = self.built_tx.clone();
        std::thread::Builder::new()
            .name("graph".into())
            .spawn(move || {
                let (input, nodes) = input_of(&g, filter, color_by, &fresh_nodes);
                let cosmos = (form == Form::Universe).then(|| graph::cosmos::build(&input));
                let scene = graph::build(form, &input);
                let _ = tx.send(Built { ticket, scene, cosmos, fresh: input.fresh.clone(), nodes, links: input.links.len(), themes: input.themes, graph: g });
            })
            .expect("graph thread");
    }

    fn install(&mut self, b: Built) {
        let Built { scene, cosmos, fresh, nodes, themes, links, graph, .. } = b;
        self.items = nodes.iter().enumerate().map(|(i, &n)| (graph.nodes[n as usize].id.clone(), i as u32)).collect();
        self.drawn = graph;
        self.nodes = nodes;
        self.theme_names = themes;
        self.links = links;
        self.births.clear();
        self.pulses.clear();
        self.last_read = None;
        self.focus_shown = None;
        self.drag = None;
        self.arcs = Arcs::default();
        self.arc_parts.clear();
        self.arcs_dirty = true;
        self.fx = Plan::default();
        self.focus_item = None;
        self.focus_amt = 0.0;
        self.hover = None;
        if let Some(c) = cosmos {
            let open = std::mem::take(&mut self.want_open) && !self.reduce_motion;
            self.settled_births = fresh.is_empty();
            self.universe.install(c, open, &fresh);
            self.scene = None;
            self.dirty = true;
            return;
        }
        self.universe.clear();
        let gap = [1.15, 0.9, 0.15][scene.tier];
        let mut plan = Plan::default();
        for (k, c) in scene.created.iter().enumerate() {
            let start = self.clock + 0.3 + k as f32 * gap;
            let life = scene.grow(c.item, start, &mut plan);
            self.births.push(Birth { start, ready: start + life, item: c.item, index_end: c.index_end, dot_end: c.dot_end });
        }
        self.settled_births = self.births.is_empty();
        self.fx = plan;
        self.fx_dirty = true;
        if self.want_open {
            self.want_open = false;
            self.open_at = if self.reduce_motion { None } else { Some(self.clock) };
            self.labels_alpha = 0.0;
        }
        self.scene = Some(Arc::new(scene));
        self.uploaded = false;
        self.dirty = true;
    }

    /// Fora do quadro de pintura: recebe geometria pronta e prepara a GPU.
    /// Devolve a textura quando ela trocou, para a cena registrar de novo.
    pub fn prepare(&mut self, gpu: &Gpu) -> (Option<&TextureView>, bool) {
        let mut arrived = false;
        while let Ok(b) = self.built_rx.try_recv() {
            if b.ticket == self.ticket {
                self.pending = false;
                self.install(b);
                arrived = true;
            }
        }
        if self.wanted.0 == 0 || self.wanted.1 == 0 {
            return (None, arrived);
        }
        let pipes = self.pipes.get_or_insert_with(|| Pipes::new(gpu));
        if let (false, Some(s)) = (self.uploaded, &self.scene) {
            pipes.vertices.write(gpu, bytemuck::cast_slice(&s.vertices));
            pipes.indices.write(gpu, bytemuck::cast_slice(&s.indices));
            pipes.dots.write(gpu, bytemuck::cast_slice(&s.dots));
            pipes.backdrop.write(gpu, bytemuck::cast_slice(&s.backdrop));
            self.uploaded = true;
            self.dirty = true;
        }
        if self.target.as_ref().is_none_or(|t| t.size != self.wanted) {
            self.target = Some(Pipes::target(gpu, self.wanted.0, self.wanted.1));
            self.dirty = true;
            return (self.target.as_ref().map(|t| &t.view), arrived);
        }
        (None, arrived)
    }

    fn hidden(&self) -> Vec<bool> {
        let Some(s) = &self.scene else { return vec![] };
        let mut v = vec![false; s.items.len()];
        for b in &self.births {
            if b.start > self.clock {
                v[b.item as usize] = true;
            }
        }
        v
    }

    fn set_focus(&mut self, item: Option<u32>) {
        if item == self.focus_item {
            return;
        }
        self.focus_item = item;
        self.focus_at = self.clock;
        if let (Some(i), Some(s)) = (item, self.scene.clone()) {
            let mut p = Plan::default();
            s.reach(i, 0.0, FOCUS_SECONDS, &mut p);
            self.focus = p;
            self.focus_shown = item;
            if s.form == Form::Roots {
                self.rebuild_arcs();
            }
        }
        self.dirty = true;
    }

    fn busy(&self) -> bool {
        let opening = self.scene.as_ref().zip(self.open_at).is_some_and(|(s, at)| self.clock - at < s.open_seconds);
        opening || !self.births.is_empty() || self.pulses.iter().any(|p| self.clock - p.start < PULSE_LIFE)
    }

    /// Avança a animação, trata o mouse e desenha nomes. Devolve o item clicado.
    pub fn paint(&mut self, ui: &mut Ui<'_>, r: Rect, image: Option<usize>, interactive: bool) -> Option<u32> {
        // Volume muito grande: menos pixels por quadro mantêm a animação fluida.
        let device = if self.nodes.len() > 6000 { ui.t.scale.min(1.5) } else { ui.t.scale };
        self.wanted = (((r.w * device) as u32).max(1), ((r.h * device) as u32).max(1));
        if self.is_dark_changed() {
            self.dirty = true;
        }
        if self.target.as_ref().is_some_and(|t| t.size != self.wanted) {
            ui.animating = true;
        }
        if let Some(img) = image {
            ui.s.image(img, r, Color::rgba(1.0, 1.0, 1.0, 1.0));
        }
        if self.pending {
            ui.animating = true;
        }
        if self.universe.active() {
            let clicked = self.universe.paint(ui, r, self.selected, interactive);
            self.hover = self.universe.hover;
            if self.universe.births_done() && !self.settled_births {
                self.settled_births = true;
                self.rebuild(vec![]);
            }
            return clicked.map(|c| c.unwrap_or(u32::MAX));
        }
        let busy = self.busy();
        if busy {
            ui.animating = true;
        }
        if busy {
            self.clock += ui.dt;
            self.dirty = true;
        }
        let mut clicked = None;
        let scene = self.scene.clone()?;
        self.stage = (r.w, r.h);
        if interactive {
            let (mx, my) = ui.st.mouse;
            let inside = ui.hover(r);
            let stage = crate::ui::id("graph-stage");
            let before = (self.zoom, self.pan);
            if inside && ui.st.clicked {
                ui.st.active = Some(stage);
                self.drag = Some(Drag { last: (mx, my), travelled: 0.0 });
                // Dois cliques voltam ao desenho inteiro.
                if ui.st.clicks >= 2 {
                    self.zoom = 1.0;
                    self.pan = (0.0, 0.0);
                }
            }
            if let (Some(d), true) = (&mut self.drag, ui.st.down) {
                let (dx, dy) = (mx - d.last.0, my - d.last.1);
                d.last = (mx, my);
                d.travelled += dx.abs() + dy.abs();
                // Só há para onde arrastar quando o desenho está maior que o quadro.
                if self.zoom > 1.001 {
                    self.pan = (self.pan.0 + dx, self.pan.1 + dy);
                }
            }
            let wheel = if inside { ui.st.wheel.1 * 0.0022 + ui.st.pinch * 1.4 } else { 0.0 };
            if wheel != 0.0 {
                // O ponto sob o mouse fica onde está: é ele que a pessoa quer ver de perto.
                let to = (self.zoom * wheel.exp()).clamp(1.0, MAX_ZOOM);
                let k = to / self.zoom;
                let (ox, oy) = (mx - (r.x + r.w * 0.5), my - (r.y + r.h * 0.5));
                self.pan = (ox - (ox - self.pan.0) * k, oy - (oy - self.pan.1) * k);
                self.zoom = to;
            }
            let (lx, ly) = ((self.zoom - 1.0) * r.w * 0.5, (self.zoom - 1.0) * r.h * 0.5);
            self.pan = (self.pan.0.clamp(-lx, lx), self.pan.1.clamp(-ly, ly));
            let moved = before != (self.zoom, self.pan);
            if moved {
                self.dirty = true;
            }
            let (s, cx, cy) = self.frame_of(r);
            let dragging = self.drag.as_ref().is_some_and(|d| d.travelled >= 4.0);
            if !inside || dragging {
                self.hover = None;
            } else if (mx, my) != self.last_mouse || ui.st.clicked || moved {
                self.last_mouse = (mx, my);
                self.hover = scene.nearest((mx - cx) / s, (my - cy) / s, 10.0 / s, &self.hidden());
            }
            if ui.st.released && ui.st.active == Some(stage) {
                ui.st.active = None;
                // Arrastar não é clicar.
                if self.drag.take().is_some_and(|d| d.travelled < 4.0) && inside {
                    clicked = Some(self.hover);
                }
            }
            let target = self.hover.or(self.selected);
            self.set_focus(target);
        }
        let (s, cx, cy) = self.frame_of(r);
        let goal = self.focus_item.is_some() as u32 as f32;
        if (self.focus_amt - goal).abs() > 0.001 {
            let step = ui.dt * 7.0;
            self.focus_amt = if goal > self.focus_amt { (self.focus_amt + step).min(1.0) } else { (self.focus_amt - step).max(0.0) };
            self.dirty = true;
            ui.animating = true;
        }
        if self.focus_item.is_some() && self.clock - self.focus_at < FOCUS_SECONDS + 0.05 {
            self.clock += ui.dt;
            self.dirty = true;
            ui.animating = true;
        }
        let reading_goal = self.pulses.iter().any(|p| self.clock - p.start < PULSE_LIFE - 1.2) as u32 as f32;
        if (self.reading - reading_goal).abs() > 0.002 {
            self.reading += (reading_goal - self.reading) * (1.0 - (-ui.dt * 3.0).exp());
            self.dirty = true;
            ui.animating = true;
        } else {
            self.reading = reading_goal;
        }
        self.settle_births();
        // Nomes dos temas: entram junto com o desenho e ficam legíveis sobre as linhas.
        let goal = self.open_at.is_none_or(|at| self.clock - at > 0.4) as u32 as f32;
        if (self.labels_alpha - goal).abs() > 0.01 {
            self.labels_alpha += (goal - self.labels_alpha) * (1.0 - (-ui.dt * 7.0).exp());
            ui.animating = true;
        } else {
            self.labels_alpha = goal;
        }
        // Com zoom, o nome de um tema pode cair fora do quadro: não invade o resto da tela.
        ui.s.push_clip(r);
        for l in &scene.labels {
            let st = if l.minor { ui_style(11.0).color(THEME.mute.alpha(self.labels_alpha)) } else { ui_style(12.0).weight(500).color(THEME.sub.alpha(self.labels_alpha)) };
            let lay = ui.line(&l.text, st);
            let (mut x, y) = (cx + l.x * s - lay.width * l.anchor, cy + l.y * s - lay.height * 0.5);
            // Com o desenho inteiro à vista, nome comprido de tema encosta na borda em vez de ser cortado.
            if self.zoom <= 1.001 {
                x = x.clamp(r.x + 8.0, (r.right() - 8.0 - lay.width).max(r.x + 8.0));
            }
            for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                ui.s.text_tinted(ui.t, &lay, x + dx, y + dy, Some(THEME.bg), 0.9 * self.labels_alpha);
            }
            ui.draw(&lay, x, y);
        }
        ui.s.pop_clip();
        clicked.map(|c| c.unwrap_or(u32::MAX))
    }

    fn settle_births(&mut self) {
        if self.births.is_empty() || self.births.iter().any(|b| self.clock < b.ready) {
            return;
        }
        if !self.settled_births {
            self.settled_births = true;
            // Assentou: refaz sem marcas de novidade, e as ligações entre novas aparecem.
            self.rebuild(vec![]);
        }
    }

    fn is_dark_changed(&mut self) -> bool {
        let d = is_dark();
        let changed = d != self.dark_seen;
        self.dark_seen = d;
        changed
    }

    /// Desenha na textura só quando algo mudou: parado, a GPU descansa.
    pub fn render(&mut self, gpu: &Gpu, enc: &mut CommandEncoder, device: f32) {
        if self.universe.active() {
            if let Some(target) = &self.target {
                let force = std::mem::take(&mut self.dirty);
                self.universe.render(gpu, enc, target, force);
            }
            return;
        }
        if !self.dirty {
            return;
        }
        let (Some(pipes), Some(target), Some(s)) = (&mut self.pipes, &self.target, &self.scene) else { return };
        if !self.uploaded {
            return;
        }
        self.dirty = false;
        let t = self.clock;
        // Plano do efeito: nascimentos em curso mais leituras recentes.
        if self.fx_dirty {
            let mut plan = Plan::default();
            for b in &self.births {
                s.grow(b.item, b.start, &mut plan);
            }
            for p in &self.pulses {
                s.reach(p.item, p.start, PULSE_SECONDS, &mut plan);
            }
            self.fx = plan;
            pipes.fx.write(gpu, bytemuck::cast_slice(&self.fx.indices));
            self.fx_dirty = false;
        }
        pipes.focus.write(gpu, bytemuck::cast_slice(&self.focus.indices));
        let mut base_i = s.base_indices;
        let mut base_d = s.base_dots;
        for b in &self.births {
            if t >= b.ready {
                base_i = b.index_end;
            }
            if t >= b.start + 0.1 {
                base_d = b.dot_end;
            }
        }
        let open = self.open_at.filter(|at| t - at < s.open_seconds);
        let open_p = open.map_or(9.0, |at| (t - at) / s.open_seconds * (1.0 + s.span * 0.3));
        let dim = (1.0 - 0.68 * self.reading) * (1.0 - 0.72 * smooth(self.focus_amt));
        let hi_a = if s.tier == 2 { 0.85 } else { 1.0 };
        let mut calls: Vec<(Call, DrawU)> = Vec::with_capacity(20);
        // De trás para a frente: fundo, brilho das linhas, linhas, destaques, halos e miolos.
        let dots_p = open.map_or(9.0, |at| (t - at) / s.open_seconds);
        calls.push((Call::Backdrop(s.backdrop.len() as u32), DrawU { p: dots_p, dim: dim.max(0.45), soft: 1.0, ..Default::default() }));
        // Em volume grande o brilho dobraria o custo de preencher a tela: lá as linhas vão sem ele.
        if GLOW_ALPHA[s.tier] > 0.0 {
            calls.push((Call::BaseGlow(base_i), DrawU { p: open_p, use_delay: 1.0, hi: 1.0, hi_a: GLOW_ALPHA[s.tier], add: GLOW_WIDTH * device, dim, soft: 1.0, ..Default::default() }));
        }
        calls.push((Call::Base(base_i), DrawU { p: open_p, use_delay: 1.0, hi_a: 1.0, dim, head_gain: if open.is_some() { 1.6 } else { 0.0 }, ..Default::default() }));
        let push = |parts: &[graph::Part], clock: f32, focus: Option<f32>, calls: &mut Vec<(Call, DrawU)>| {
            for p in parts {
                if calls.len() + 3 >= MAX_DRAWS {
                    break;
                }
                let age = clock - p.start;
                if age < 0.0 {
                    continue;
                }
                let r = (age / p.duration).clamp(0.0, 1.0);
                let after = ((age - p.duration) / p.fade).clamp(0.0, 1.0);
                if focus.is_none() && p.rest == 0.0 && after >= 1.0 {
                    continue;
                }
                let keep = focus.map_or(p.hi * (1.0 + (p.rest - 1.0) * smooth(after)), |c| p.hi * c);
                let u = DrawU { p: if r >= 1.0 { 9.0 } else { p.from + (p.to - p.from) * ease_out(r) }, rev: p.rev as u32 as f32, hi: keep, hi_a, add: p.widen * device * keep, head_gain: 1.0 - r, dim: 1.0, ..Default::default() };
                let call = if focus.is_some() { Call::Focus { offset: p.offset, count: p.count } } else { Call::Fx { offset: p.offset, count: p.count } };
                calls.push((call, u));
            }
        };
        push(&self.fx.parts, t, None, &mut calls);
        if self.focus_amt > 0.0 {
            push(&self.focus.parts, t - self.focus_at, Some(smooth(self.focus_amt)), &mut calls);
        }
        if self.arcs_dirty {
            pipes.arc_vertices.write(gpu, bytemuck::cast_slice(&self.arcs.vertices));
            pipes.arc_indices.write(gpu, bytemuck::cast_slice(&self.arcs.indices));
            self.arcs_dirty = false;
        }
        let mut lights: Vec<Dot> = Vec::new();
        let light = |item: u32, grow: f32, alpha: f32, lights: &mut Vec<Dot>| {
            let it = &s.items[item as usize];
            lights.push(Dot { pos: [it.x, it.y], size: [11.0, 9.0, 7.0][s.tier] * grow, delay: -1.0, alpha, theme: it.theme as f32, kind: 0.0 });
        };
        for part in &self.arc_parts {
            if calls.len() + 3 >= MAX_DRAWS {
                break;
            }
            match *part {
                ArcPart::Read { span, start, from, to } => {
                    let r = (t - (start - TRAVEL)) / TRAVEL;
                    let since = t - start;
                    let fade = (1.0 - (since - (PULSE_LIFE - 1.2)) / 1.2).clamp(0.0, 1.0);
                    if r < 0.0 || fade <= 0.0 {
                        continue;
                    }
                    let travelling = r < 1.0;
                    let p = if travelling { smooth(r) } else { 9.0 };
                    calls.push((Call::Arc { offset: span.0, count: span.1 - span.0 }, DrawU { p, hi: 1.0, hi_a: 0.85 * fade, add: 1.3 * device, head_gain: if travelling { 1.5 } else { 0.0 }, dim: 1.0, ..Default::default() }));
                    if travelling {
                        // A ponta da luz, e de onde ela saiu.
                        let head = s.arc_at(from, to, p);
                        lights.push(Dot { pos: head, size: [9.0, 7.5, 6.0][s.tier], delay: -1.0, alpha: 1.0, theme: s.items[to as usize].theme as f32, kind: 0.0 });
                        light(from, 1.2, 0.7 * (1.0 - r), &mut lights);
                    }
                }
                ArcPart::Focus { span, to } => {
                    let a = smooth(self.focus_amt);
                    if a <= 0.0 {
                        continue;
                    }
                    let r = ((t - self.focus_at) / FOCUS_SECONDS).clamp(0.0, 1.0);
                    let p = if r >= 1.0 { 9.0 } else { ease_out(r) };
                    calls.push((Call::Arc { offset: span.0, count: span.1 - span.0 }, DrawU { p, hi: 1.0, hi_a: 0.7 * a, add: 0.5 * device, dim: 1.0, ..Default::default() }));
                    light(to, 0.8, 0.75 * a * r, &mut lights);
                }
            }
        }
        calls.push((Call::Halos(base_d), DrawU { p: dots_p, dim: dim.max(0.5), soft: 1.0, hi_a: HALO_ALPHA, add: HALO_GROW, ..Default::default() }));
        calls.push((Call::Dots(base_d), DrawU { p: dots_p, dim: dim.max(0.5), ..Default::default() }));
        let mut glows: Vec<Dot> = Vec::with_capacity(64);
        for g in &self.fx.glows {
            let a = (t - g.start) / 1.1;
            if (0.0..1.0).contains(&a) {
                let e = 1.0 - a;
                glows.push(Dot { pos: [g.x, g.y], size: g.size * (0.6 + 0.6 * ease_out(a)), delay: -1.0, alpha: e * e * 0.9, theme: g.theme as f32, kind: 0.0 });
            }
        }
        if self.focus_amt > 0.0 {
            for g in &self.focus.glows {
                glows.push(Dot { pos: [g.x, g.y], size: g.size * 0.7, delay: -1.0, alpha: 0.8 * self.focus_amt, theme: g.theme as f32, kind: 0.0 });
            }
        }
        glows.extend(lights);
        glows.truncate(6000);
        if !glows.is_empty() {
            pipes.glows.write(gpu, bytemuck::cast_slice(&glows));
            calls.push((Call::Glows(glows.len() as u32), DrawU { p: 9.0, dim: 1.0, soft: 1.0, ..Default::default() }));
        }
        // O alvo tem os pixels do quadro: o arrasto, medido em pontos, vira pixels na mesma proporção.
        let fit = Scene::fit(target.size.0 as f32, target.size.1 as f32);
        let per_point = target.size.0 as f32 / self.stage.0.max(1.0);
        let light = !is_dark();
        let mut pal = [[0.0f32; 4]; SLOTS + 1];
        for (i, c) in THEME.themes.iter().enumerate() {
            pal[i] = [c.r, c.g, c.b, 1.0];
        }
        pal[SLOTS] = [THEME.ink.r, THEME.ink.g, THEME.ink.b, 1.0];
        let frame = FrameU {
            res: [target.size.0 as f32, target.size.1 as f32],
            scale: fit * self.zoom,
            light: light as u32 as f32,
            span: s.span,
            head: s.head,
            offset: [self.pan.0 * per_point, self.pan.1 * per_point],
            wscale: fit * self.zoom.sqrt(),
            pad: [0.0; 3],
            pal,
        };
        pipes.draw(gpu, enc, target, frame, [THEME.bg.r, THEME.bg.g, THEME.bg.b], &calls);
    }

    /// Ficha do item sob o mouse. Os textos vêm de quem conhece o modelo.
    pub fn tooltip(&self, ui: &mut Ui<'_>, r: Rect, item: u32, title: &str, meta: &str) {
        let Some((px, py)) = self.screen_of(item, r) else { return };
        let st = ui_style(13.0).weight(600);
        let title = ui.ellipsize(title, st, 300.0);
        let (t1, t2) = (ui.line(&title, st), ui.line(meta, mono(11.0)));
        let w = t1.width.max(t2.width) + 22.0;
        let h = 50.0;
        let x = if px > r.x + r.w * 0.5 { px - w - 12.0 } else { px + 12.0 };
        let y = if py > r.y + r.h * 0.5 { py - h - 12.0 } else { py + 12.0 };
        let card = Rect::new(x.clamp(r.x + 6.0, r.right() - w - 6.0), y, w, h);
        ui.s.shadow(card, 8.0, 18.0, THEME.shadow);
        ui.s.panel(card, 8.0, THEME.panel, THEME.line, 1.0);
        ui.draw(&t1, card.x + 11.0, card.y + 7.0);
        ui.draw(&t2, card.x + 11.0, card.y + 27.0);
    }
}

impl Default for GraphView {
    fn default() -> Self {
        Self::new()
    }
}

/// Fatia de cor de cada grupo, os maiores primeiro. Do oitavo em diante vira "Outros".
pub fn theme_slots(g: &Graph) -> (Vec<u8>, Vec<String>) {
    let of: Vec<u32> = g.nodes.iter().map(|n| n.region).collect();
    slots_of(&of, &g.regions)
}

/// Fatias de cor para uma categoria qualquer: `of` diz a de cada nó, `names` o nome de cada uma.
fn slots_of(of: &[u32], names: &[String]) -> (Vec<u8>, Vec<String>) {
    let mut size = vec![0usize; names.len()];
    for &c in of {
        if let Some(n) = size.get_mut(c as usize) {
            *n += 1;
        }
    }
    let mut ranked: Vec<usize> = (0..size.len()).filter(|&i| size[i] > 0).collect();
    ranked.sort_by(|&a, &b| size[b].cmp(&size[a]).then(names[a].cmp(&names[b])));
    let overflow = ranked.len() > SLOTS;
    let mut slot = vec![(SLOTS - 1) as u8; size.len()];
    let named = if overflow { SLOTS - 1 } else { ranked.len() };
    for (i, &r) in ranked.iter().take(named).enumerate() {
        slot[r] = i as u8;
    }
    let mut out: Vec<String> = ranked.iter().take(named).map(|&r| names[r].clone()).collect();
    if overflow {
        out.push("Outros".into());
    }
    (slot, out)
}

/// Monta a entrada da geometria: filtro, fatias de tema e quem é novo.
fn input_of(g: &Graph, filter: Filter, color_by: ColorBy, fresh_nodes: &[u32]) -> (Input, Vec<u32>) {
    // Por projeto, quem não tem projeto forma a sua própria fatia.
    let mut project_names = g.projects.clone();
    project_names.push("Sem projeto".into());
    let none = project_names.len() as u32 - 1;
    let by_project: Vec<u32> = g.nodes.iter().map(|n| if n.project == u32::MAX { none } else { n.project }).collect();
    let by_group: Vec<u32> = g.nodes.iter().map(|n| n.region).collect();
    let (slot, themes) = match color_by {
        ColorBy::Group => slots_of(&by_group, &g.regions),
        ColorBy::Project => slots_of(&by_project, &project_names),
    };
    let category = if color_by == ColorBy::Group { &by_group } else { &by_project };
    let max_w = g.nodes.iter().map(|n| n.weight).fold(1.0f32, f32::max);
    let is_fresh: FxHashSet<u32> = fresh_nodes.iter().copied().collect();
    let mut nodes = vec![];
    let mut map = vec![u32::MAX; g.nodes.len()];
    let mut back = vec![];
    for (i, n) in g.nodes.iter().enumerate() {
        if !filter.keeps(n.kind) {
            continue;
        }
        map[i] = nodes.len() as u32;
        back.push(i as u32);
        let theme = slot.get(category[i] as usize).copied().unwrap_or((SLOTS - 1) as u8);
        nodes.push(Node { theme, sub: 0, uses: (n.weight / max_w).clamp(0.0, 1.0).sqrt(), fresh: is_fresh.contains(&(i as u32)), key: key_of(&n.id), note: matches!(n.kind, NodeKind::Note), hub: n.hub });
    }
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();
    let (mut old, mut new) = (vec![], vec![]);
    for &(a, b, w) in &g.edges {
        let (x, y) = (map[a as usize], map[b as usize]);
        if x == u32::MAX || y == u32::MAX || x == y || !seen.insert((x.min(y), x.max(y))) {
            continue;
        }
        match (nodes[x as usize].fresh, nodes[y as usize].fresh) {
            (false, false) => old.push((x, y, w)),
            (true, true) => {}
            _ => new.push((x, y, w)),
        }
    }
    let base_links = old.len();
    old.extend(new);
    let weights: Vec<f32> = old.iter().map(|l| l.2).collect();
    let old: Vec<(u32, u32)> = old.iter().map(|l| (l.0, l.1)).collect();
    assign_subs(&mut nodes, &old);
    // Com a cor no grupo, o subtema é o projeto: os itens de um projeto ficam juntos dentro do grupo.
    let mut sub_names = Vec::new();
    if color_by == ColorBy::Group {
        let mut seen: FxHashSet<u32> = FxHashSet::default();
        for (k, &i) in back.iter().enumerate() {
            let p = g.nodes[i as usize].project;
            if p == u32::MAX {
                continue;
            }
            nodes[k].sub = PROJECT_SUB + p;
            if seen.insert(p) {
                sub_names.push((PROJECT_SUB + p, g.projects.get(p as usize).cloned().unwrap_or_default()));
            }
        }
    }
    let fresh: Vec<u32> = fresh_nodes.iter().filter_map(|&n| Some(map[n as usize]).filter(|&m| m != u32::MAX)).collect();
    (Input { nodes, links: old, weights, base_links, fresh, themes, sub_names }, back)
}
