//! A casca do app: barra lateral, páginas, painel de detalhe e boas-vindas.

use crate::command::{Cmd, Msg};
use crate::graphview::{ColorBy, Filter, GraphView};
use crate::input::{Event, Key};
use crate::model::{Graph, GraphNode, Live, MemKind, MemState, Memory, Model, NodeKind, Snapshot};
use crate::theme::{self, mono, ui as ui_style, THEME};
use crate::ui::{id, Ui, UiState};
use crate::views::activity::ActivityView;
use crate::views::memories::{self, MemoriesView};
use crate::views::metrics::MetricsView;
use crate::views::notes::NotesView;
use crate::views::palette::{Action, Choice, ChoiceKind, ChoiceOption, Palette, Picked, Prompt};
use crate::views::profile::ProfileView;
use crate::views::settings::SettingsView;
use crate::Clipboard;
use gfx::wgpu::{CommandEncoder, TextureView};
use gfx::{Globals, Gpu, Rect, Renderer, Scene, TextEngine};
use graph::Form;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Welcome,
    Notes,
    Memories,
    Graph,
    Activity,
    Metrics,
    Profile,
    Settings,
}

const NAV: [(Page, &str); 6] = [(Page::Notes, "Notas"), (Page::Memories, "Memórias"), (Page::Graph, "Grafo"), (Page::Activity, "Atividade"), (Page::Metrics, "Métricas"), (Page::Profile, "Perfil")];
const SIDE_W: f32 = 232.0;
const PANEL_W: f32 = 380.0;

pub struct FrameOut {
    pub animating: bool,
    pub drag_window: bool,
    pub ime_area: Option<(f32, f32, f32)>,
    pub wake_in: Option<std::time::Duration>,
}

pub struct App {
    pub st: UiState,
    pub scene: Scene,
    pub text: TextEngine,
    pub renderer: Renderer,
    image: Option<usize>,
    pub page: Page,
    pub model: Snapshot,
    pub graph: GraphView,
    pub notes: NotesView,
    memories: MemoriesView,
    activity: ActivityView,
    metrics: MetricsView,
    pub profile: ProfileView,
    settings: SettingsView,
    pub palette: Palette,
    pub clip: Clipboard,
    tx: Sender<Cmd>,
    rx: Receiver<Msg>,
    time: f32,
    live: Option<(String, f32)>,
    toast: Option<(String, f32)>,
    fade: f32,
    pub quit_ready: bool,
    help: bool,
    /// Memória aberta no painel lateral.
    pub detail: Option<String>,
    detail_body: Option<(String, String, String)>,
    detail_asked: (String, String),
    detail_height: f32,
    /// Onde está o botão Começar: o teste de primeira abertura clica nele.
    pub start_button: Rect,
    graph_gen: u64,
    theme_chosen: Option<bool>,
    /// Quadros ainda devidos depois de uma entrada. O clique é tratado enquanto a tela é
    /// pintada: o que ele muda só aparece no quadro seguinte, e ninguém mais pediria esse quadro.
    follow: u8,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(gpu: &Gpu, _w: u32, _h: u32, _scale: f32, text: TextEngine, tx: Sender<Cmd>, rx: Receiver<Msg>, clip: Clipboard) -> Self {
        let renderer = Renderer::new(gpu);
        let theme_chosen = crate::data::load_theme();
        if let Some(d) = theme_chosen {
            theme::set_dark(d);
        }
        let mut graph = GraphView::new();
        graph.reduce_motion = crate::platform::reduce_motion();
        Self {
            st: UiState::default(),
            scene: Scene::new(),
            text,
            renderer,
            image: None,
            page: Page::Graph,
            model: Arc::new(Model::default()),
            graph,
            notes: NotesView::new(),
            memories: MemoriesView::default(),
            activity: ActivityView::default(),
            metrics: MetricsView::default(),
            profile: ProfileView::default(),
            settings: SettingsView::default(),
            palette: Palette::new(),
            clip,
            tx,
            rx,
            time: 0.0,
            live: None,
            toast: None,
            fade: 0.0,
            quit_ready: false,
            help: false,
            detail: None,
            detail_body: None,
            detail_asked: (String::new(), String::new()),
            detail_height: 0.0,
            start_button: Rect::default(),
            graph_gen: 0,
            theme_chosen,
            follow: 0,
        }
    }

    /// Aparência do sistema. Vale até a pessoa escolher em Ajustes.
    pub fn system_theme(&mut self, dark: bool) {
        if self.theme_chosen.is_none() {
            theme::set_dark(dark);
        }
    }

    pub fn resize(&mut self, _gpu: &Gpu, _w: u32, _h: u32) {}

    pub fn show_note(&mut self, path: String, text: String, hash: String) {
        self.notes.opened(path, text, hash, &self.tx);
    }

    pub fn open_note(&mut self, path: &str) {
        self.notes.request_open(path, &self.tx);
    }

    pub fn go(&mut self, p: Page) {
        if self.page != p {
            if self.page == Page::Profile {
                self.profile.commit(&self.tx);
            }
            self.page = p;
            self.fade = 1.0;
            if !matches!(p, Page::Memories | Page::Graph | Page::Notes | Page::Metrics) {
                self.detail = None;
            }
            self.graph.visible = p == Page::Graph;
            if p == Page::Graph {
                self.graph.replay();
            }
        }
    }

    /// Grava tudo antes de sair. Único caminho de saída, seja qual for o gatilho.
    pub fn flush_all(&mut self) {
        self.profile.commit(&self.tx);
        self.notes.flush_for_exit(&self.tx);
        let _ = self.tx.send(Cmd::Shutdown);
    }

    pub fn pump(&mut self) -> bool {
        let mut any = false;
        while let Ok(m) = self.rx.try_recv() {
            any = true;
            self.follow = self.follow.max(1);
            match m {
                Msg::Snapshot(s) => self.install(s),
                Msg::Note { path, text, hash } => {
                    self.notes.opened(path, text, hash, &self.tx);
                    self.go(Page::Notes);
                }
                Msg::Saved { path, hash } => self.notes.saved(&path, hash, &self.tx),
                Msg::Conflict { path, disk, hash } => self.notes.conflict(&path, disk, hash),
                Msg::SaveFailed { path, reason } => {
                    self.notes.save_failed(&path);
                    self.toast = Some((format!("Não consegui salvar ({reason}). O texto segue aqui e vou tentar de novo."), self.time));
                }
                Msg::External { path, text, hash } => self.notes.external(&path, text, hash),
                Msg::Live(l) => self.live_event(l),
                Msg::Results { seq, hits, note } => self.palette.results(seq, hits, note),
                Msg::MemoryBody { id, hash, body } => self.detail_body = Some((id, hash, body)),
                Msg::Toast(t) => self.toast = Some((t, self.time)),
                Msg::NoBrain => {
                    self.page = Page::Welcome;
                    self.graph.visible = true;
                    self.graph.form = Form::Roots;
                    self.graph.set_graph(Arc::new(demo_graph()));
                }
            }
        }
        any
    }

    fn live_event(&mut self, l: Live) {
        if let Live::Read { node, .. } = &l {
            self.graph.live_read(node);
        }
        if let Live::SessionEnded = l {
            if let Some(st) = self.model.sessions.first() {
                let t = match st.changes.len() {
                    0 => "A sessão terminou sem trazer memórias.".to_string(),
                    1 => "A sessão trouxe 1 memória.".to_string(),
                    n => format!("A sessão trouxe {n} memórias."),
                };
                self.toast = Some((t, self.time));
            }
        }
        let line = match &l {
            Live::Read { node, client } => Some(format!("{client} leu {}", self.title_of(node))),
            Live::Searched { query, client, found } => Some(format!("{client} buscou “{query}”, {found} achados")),
            Live::Wrote { title, client, .. } => Some(format!("{client} aprendeu: {title}")),
            Live::Undid { node } => Some(format!("Desfeito: {}", self.title_of(node))),
            Live::SessionEnded => None,
        };
        if let Some(t) = line {
            self.live = Some((t, self.time));
        }
    }

    fn title_of(&self, node: &str) -> String {
        self.model.graph.nodes.iter().find(|n| n.id == node).map_or_else(|| node.to_string(), |n| n.title.clone())
    }

    fn install(&mut self, s: Snapshot) {
        let welcome = self.page == Page::Welcome;
        if welcome {
            // Saindo das boas-vindas: o grafo de fundo era de mentira e não conta como conhecido.
            self.graph = {
                let mut g = GraphView::new();
                g.reduce_motion = self.graph.reduce_motion;
                g
            };
            self.image = None;
            self.page = Page::Graph;
            self.graph_gen = 0;
        }
        if self.graph_gen != s.generation {
            self.graph_gen = s.generation;
            self.graph.set_graph(Arc::new(s.graph.clone()));
        }
        self.model = s;
    }

    pub fn event(&mut self, e: Event) {
        // Mexer o mouse só precisa de um quadro a mais; o resto pode trocar de tela no meio da pintura.
        self.follow = self.follow.max(if matches!(e, Event::MouseMove(..)) { 1 } else { 2 });
        match &e {
            Event::MouseMove(x, y) => {
                self.st.mouse = (*x, *y);
                return;
            }
            Event::MouseDown(x, y, clicks) => {
                self.st.mouse = (*x, *y);
                self.st.down = true;
                self.st.clicked = true;
                self.st.clicks = *clicks;
                return;
            }
            Event::MouseUp(x, y) => {
                self.st.mouse = (*x, *y);
                self.st.down = false;
                self.st.released = true;
                return;
            }
            Event::Scroll(x, y) => {
                self.st.wheel.0 += x;
                self.st.wheel.1 += y;
                return;
            }
            Event::Pinch(d) => {
                self.st.pinch += d;
                return;
            }
            Event::Focus(false) => {
                self.notes.flush(&self.tx);
                return;
            }
            Event::Focus(true) => return,
            Event::Key(_, m) => self.st.mods = *m,
            _ => {}
        }
        if self.help {
            if matches!(e, Event::Key(..)) {
                self.help = false;
            }
            return;
        }
        if self.palette.open {
            self.palette_event(&e);
            return;
        }
        if let Some((_, b)) = &mut self.profile.editing {
            match &e {
                Event::Key(Key::Enter, _) => self.profile.commit(&self.tx),
                Event::Key(Key::Escape, _) => self.profile.editing = None,
                Event::Key(Key::Backspace, _) => b.backspace(),
                Event::Key(Key::Left, _) => b.motion(ed::Move::Left, false),
                Event::Key(Key::Right, _) => b.motion(ed::Move::Right, false),
                Event::Text(t) => {
                    b.preedit.clear();
                    b.insert(t);
                }
                _ => {}
            }
            return;
        }
        if let Event::Key(k, m) = &e {
            if m.cmd {
                match k {
                    Key::Char('k') => return self.palette.show(),
                    Key::Char(c @ '1'..='6') => return self.go(NAV[(*c as u8 - b'1') as usize].0),
                    Key::Char(',') => return self.go(Page::Settings),
                    Key::Char('s') => return self.notes.flush(&self.tx),
                    Key::Char('n') => return self.run_action(Action::NewNote),
                    Key::Char('d') => return self.run_action(Action::Daily),
                    Key::Char('/') => return self.run_action(Action::Help),
                    Key::Char('[') => return self.notes.back(&self.tx),
                    Key::Char(']') => return self.notes.ahead(&self.tx),
                    _ => {}
                }
            }
            if *k == Key::Escape && self.detail.is_some() {
                self.detail = None;
                return;
            }
        }
        if self.page == Page::Notes {
            self.notes.event(&e, &mut self.clip);
        }
    }

    fn palette_event(&mut self, e: &Event) {
        match e {
            Event::Key(Key::Escape, _) => self.palette.open = false,
            Event::Key(Key::Up, _) => self.palette.move_cursor(-1),
            Event::Key(Key::Down, _) => self.palette.move_cursor(1),
            Event::Key(Key::Backspace, _) => self.palette.buf.backspace(),
            Event::Key(Key::Left, _) => self.palette.buf.motion(ed::Move::Left, false),
            Event::Key(Key::Right, _) => self.palette.buf.motion(ed::Move::Right, false),
            Event::Key(Key::Enter, _) => {
                if let Some(p) = self.palette.pick() {
                    self.apply_pick(p);
                }
            }
            Event::Key(Key::Char('k'), m) if m.cmd => self.palette.open = false,
            Event::Key(Key::Char('v'), m) if m.cmd => {
                let t = self.clip.get();
                self.palette.buf.insert(&t);
            }
            Event::Text(t) => {
                self.palette.buf.preedit.clear();
                self.palette.buf.insert(t);
            }
            Event::Preedit(t) => self.palette.buf.preedit = t.clone(),
            _ => {}
        }
    }

    fn apply_pick(&mut self, p: Picked) {
        self.palette.open = false;
        match p {
            Picked::Action(a) => self.run_action(a),
            Picked::Prompted(kind, text) => {
                let text = text.trim().to_string();
                if text.is_empty() {
                    return;
                }
                match kind {
                    Prompt::NewNote => {
                        let _ = self.tx.send(Cmd::CreateNote(text));
                    }
                    Prompt::Rename => {
                        if let Some(path) = self.notes.path.clone() {
                            self.notes.flush(&self.tx);
                            let _ = self.tx.send(Cmd::RenameNote { path, title: text });
                        }
                    }
                }
            }
            Picked::Chosen(kind, value) => {
                let _ = self.tx.send(match kind {
                    ChoiceKind::MemoryProject { id, hash } => Cmd::SetProject { id, hash, project: value },
                    ChoiceKind::MoveProject { from } => Cmd::MoveProject { from, to: value },
                    ChoiceKind::ProjectGroup { slug } => match value {
                        Some(group) => Cmd::CreateProject { slug, group },
                        None => return,
                    },
                });
            }
            Picked::Hit(h) => {
                if h.is_note {
                    self.notes.request_open(&h.id, &self.tx);
                } else {
                    self.go(Page::Memories);
                    self.detail = Some(h.id);
                }
            }
        }
    }

    fn run_action(&mut self, a: Action) {
        match a {
            Action::Page(i) => self.go(NAV[i].0),
            Action::Settings => self.go(Page::Settings),
            Action::NewNote => self.palette.ask(Prompt::NewNote, "Nome da nota", ""),
            Action::Daily => {
                let _ = self.tx.send(Cmd::DailyNote);
            }
            Action::Rename => {
                if let Some(p) = self.notes.path.clone() {
                    let stem = p.rsplit('/').next().unwrap_or(&p).trim_end_matches(".md").to_string();
                    self.palette.ask(Prompt::Rename, "Novo nome", &stem);
                }
            }
            Action::Delete => {
                if let Some(p) = self.notes.path.take() {
                    self.notes.editor = None;
                    let _ = self.tx.send(Cmd::DeleteNote(p));
                }
            }
            Action::OpenBrain => {
                self.notes.flush(&self.tx);
                if let Some(p) = crate::pick_folder("Escolha a pasta do cofre") {
                    let _ = self.tx.send(Cmd::OpenBrain(p));
                }
            }
            Action::Help => self.help = !self.help,
        }
    }

    /// Para onde uma memória pode ir: geral, ou qualquer projeto do cofre menos o atual.
    fn project_options(&self, skip: Option<&str>) -> Vec<ChoiceOption> {
        let mut list = vec![ChoiceOption { label: "Geral".into(), detail: "vale em todo projeto".into(), value: None }];
        let mut known: Vec<&crate::model::ProjectInfo> = self.model.projects.iter().filter(|p| Some(p.slug.as_str()) != skip && !p.ephemeral).collect();
        known.sort_by(|a, b| a.note.is_none().cmp(&b.note.is_none()).then(a.title.to_lowercase().cmp(&b.title.to_lowercase())));
        list.extend(known.into_iter().map(|p| ChoiceOption { label: p.title.clone(), detail: if p.note.is_some() { p.group.clone() } else { "sem nota".into() }, value: Some(p.slug.clone()) }));
        list
    }

    /// Grupos que as notas de projeto já usam, os mais cheios primeiro.
    fn group_options(&self) -> Vec<ChoiceOption> {
        let mut count: Vec<(String, usize)> = Vec::new();
        for p in self.model.projects.iter().filter(|p| !p.group.is_empty()) {
            match count.iter_mut().find(|c| c.0 == p.group) {
                Some(c) => c.1 += 1,
                None => count.push((p.group.clone(), 1)),
            }
        }
        count.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        if count.is_empty() {
            count.push(("Projetos".into(), 0));
        }
        count.into_iter().map(|(g, n)| ChoiceOption { detail: if n == 1 { "1 projeto".into() } else { format!("{n} projetos") }, value: Some(g.clone()), label: g }).collect()
    }

    fn toggle_theme(&mut self) {
        let dark = !theme::is_dark();
        theme::set_dark(dark);
        self.theme_chosen = Some(dark);
        crate::data::save_theme(dark);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn frame(&mut self, gpu: &Gpu, enc: &mut CommandEncoder, view: &TextureView, lw: f32, lh: f32, scale: f32, dt: f32) -> FrameOut {
        self.pump();
        let fonts_arrived = self.text.poll_fallback();
        let graph_on = matches!(self.page, Page::Graph | Page::Welcome);
        let mut arrived = false;
        if graph_on {
            let (target, got) = self.graph.prepare(gpu);
            arrived = got;
            if let Some(v) = target {
                self.image = Some(self.renderer.register_image(gpu, v, self.image));
                arrived = true;
            }
        }
        self.time += dt;
        let time = self.time;
        self.scene.clear();
        self.st.hot = None;
        let area = Rect::new(0.0, 0.0, lw, lh);
        let mut out = FrameOut { animating: false, drag_window: false, ime_area: None, wake_in: self.notes.tick(&self.tx) };
        let mut scene = std::mem::take(&mut self.scene);
        let mut text = std::mem::replace(&mut self.text, TextEngine::new(&[], scale));
        let mut st = std::mem::take(&mut self.st);
        let mut palette_pick = None;
        {
            let mut ui = Ui { s: &mut scene, t: &mut text, st: &mut st, time, dt, size: (lw, lh), animating: false, in_overlay: false };
            ui.s.fill(area, THEME.bg);
            if self.page == Page::Welcome {
                self.welcome(&mut ui, area);
            } else {
                let main = area.pad(SIDE_W, 0.0, 0.0, 0.0);
                self.page_body(&mut ui, main);
                if std::mem::take(&mut self.notes.blocked) {
                    self.toast = Some(("Resolva o conflito desta nota antes de abrir outra.".into(), self.time));
                    self.go(Page::Notes);
                }
                if self.fade > 0.0 {
                    ui.s.fill(main, THEME.bg.alpha(self.fade));
                    self.fade = (self.fade - dt * 6.5).max(0.0);
                    ui.animating = true;
                }
                self.detail_panel(&mut ui, main);
                self.sidebar(&mut ui, Rect::new(0.0, 0.0, SIDE_W, lh));
                out.drag_window = Self::title_drag(&mut ui, lw);
            }
            if self.palette.open {
                palette_pick = self.palette.paint(&mut ui, area, &self.tx);
            }
            if self.help {
                help_sheet(&mut ui, area);
            }
            self.toasts(&mut ui, area);
            out.animating = ui.animating || fonts_arrived || arrived;
        }
        if self.follow > 0 {
            self.follow -= 1;
            out.animating = true;
        }
        st.end_frame();
        self.st = st;
        self.text = text;
        self.scene = scene;
        if let Some(p) = palette_pick {
            self.apply_pick(p);
            out.animating = true;
        }
        if self.page == Page::Notes {
            out.ime_area = self.notes.editor.as_ref().map(|e| e.caret_screen);
        }
        if graph_on {
            self.graph.render(gpu, enc, scale);
        }
        self.renderer.render(gpu, enc, view, &self.scene, &mut self.text, Globals { size: [lw, lh], scale, time }, THEME.bg);
        out
    }

    fn page_body(&mut self, ui: &mut Ui<'_>, r: Rect) {
        let m = self.model.clone();
        match self.page {
            Page::Welcome => {}
            Page::Notes => {
                let o = self.notes.paint(ui, r, &m, &self.tx);
                if o.new_note {
                    self.run_action(Action::NewNote);
                }
                if o.open_memory.is_some() {
                    self.detail = o.open_memory;
                }
            }
            Page::Memories => {
                if let Some(pick) = self.memories.paint(ui, r, &m, self.detail.as_deref()) {
                    self.detail = Some(pick);
                }
            }
            Page::Graph => self.graph_page(ui, r, &m),
            Page::Activity => {
                if self.activity.paint(ui, r, &m, &self.tx) {
                    self.go(Page::Settings);
                }
            }
            Page::Metrics => {
                let o = self.metrics.paint(ui, r, &m);
                if let Some(n) = o.open_note {
                    self.notes.request_open(&n, &self.tx);
                }
                if o.open_memory.is_some() {
                    self.detail = o.open_memory;
                }
                if o.review_held {
                    self.memories.filter = memories::Filter::Review;
                    self.go(Page::Memories);
                }
                if o.show_memories {
                    self.memories.filter = memories::Filter::All;
                    self.go(Page::Memories);
                }
                if o.show_graph {
                    self.go(Page::Graph);
                }
                if let Some(t) = o.create_note {
                    let _ = self.tx.send(Cmd::CreateNote(t));
                }
                if o.dream {
                    self.clip.set("/sonhar");
                    self.toast = Some(("/sonhar copiado. Cole na sua IA para ela revisar as memórias.".into(), self.time));
                }
            }
            Page::Profile => self.profile.paint(ui, r, &m, &self.tx),
            Page::Settings => {
                let o = self.settings.paint(ui, r, &m, &self.tx, &mut self.clip);
                if o.toggle_theme {
                    self.toggle_theme();
                }
                if o.open_vault {
                    self.run_action(Action::OpenBrain);
                }
                if let Some(slug) = o.create_project {
                    let options = self.group_options();
                    self.palette.choose(Choice { kind: ChoiceKind::ProjectGroup { slug }, label: "Grupo do projeto: escolha ou digite um novo", options, free_text: true });
                }
                if let Some(from) = o.move_project {
                    let options = self.project_options(Some(&from));
                    self.palette.choose(Choice { kind: ChoiceKind::MoveProject { from }, label: "Levar as memórias para", options, free_text: false });
                }
            }
        }
    }

    fn graph_page(&mut self, ui: &mut Ui<'_>, r: Rect, m: &Model) {
        let mut x = r.x + 20.0 + ui.head(r.x + 20.0, r.y, "Grafo") + 16.0;
        for (form, label) in [(Form::Bundles, "Feixes"), (Form::Roots, "Raízes"), (Form::Universe, "Universo")] {
            let (fired, w) = ui.chip(id(("graph-form", label)), x, r.y + 14.0, label, self.graph.form == form);
            if fired {
                self.detail = None;
                self.graph.selected = None;
                self.graph.set_form(form);
            }
            x += w + 2.0;
        }
        x += 14.0;
        for (filter, label) in [(Filter::All, "Tudo"), (Filter::Notes, "Notas"), (Filter::Memories, "Memórias")] {
            let (fired, w) = ui.chip(id(("graph-filter", label)), x, r.y + 14.0, label, self.graph.filter == filter);
            if fired {
                self.detail = None;
                self.graph.selected = None;
                self.graph.set_filter(filter);
            }
            x += w + 2.0;
        }
        x += 14.0;
        // O que dá a cor: o grupo, com cada projeto junto dentro dele, ou o projeto direto.
        for (by, label) in [(ColorBy::Group, "Grupo"), (ColorBy::Project, "Projeto")] {
            let (fired, w) = ui.chip(id(("graph-color", label)), x, r.y + 14.0, label, self.graph.color_by == by);
            if fired {
                self.detail = None;
                self.graph.selected = None;
                self.graph.set_color_by(by);
            }
            x += w + 2.0;
        }
        let stage = r.pad(0.0, 56.0, 0.0, 0.0);
        if m.graph.nodes.is_empty() {
            ui.s.fill(Rect::new(stage.x, stage.y, stage.w, 1.0), THEME.line);
            let (x, y) = (stage.x + stage.w * 0.5 - 180.0, stage.y + stage.h * 0.32);
            ui.text("Seu cofre ainda está vazio.", ui_style(22.0).weight(600), x, y);
            ui.para("O grafo cresce quando você escreve uma nota ou a IA aprende algo.", ui_style(15.0).color(THEME.sub), x, y + 38.0, 360.0);
            if ui.primary(id("empty-note"), Rect::new(x, y + 96.0, 170.0, 40.0), "Escrever uma nota") {
                self.run_action(Action::NewNote);
            }
            if ui.button(id("empty-connect"), Rect::new(x + 180.0, y + 96.0, 150.0, 40.0), "Conectar sua IA") {
                self.go(Page::Settings);
            }
            return;
        }
        self.graph.selected = self.detail.as_deref().and_then(|d| self.graph.item_of(d));
        let clicked = self.graph.paint(ui, stage, self.image, true);
        ui.s.fill(Rect::new(stage.x, stage.y, stage.w, 1.0), THEME.line);
        if let Some(item) = self.graph.hover {
            if let Some(n) = self.graph.node_of(item) {
                let kind = match n.kind {
                    NodeKind::Note => "Nota",
                    NodeKind::Memory(k) => k.label(),
                };
                let reads = m.metrics.reads_30.get(&n.id).copied().unwrap_or(0);
                let times = if reads == 1 { "lida 1 vez em 30 dias".to_string() } else { format!("lida {reads} vezes em 30 dias") };
                let (title, meta) = (n.title.clone(), format!("{kind} · {times}"));
                self.graph.tooltip(ui, stage, item, &title, &meta);
            }
        }
        match clicked {
            Some(u32::MAX) => self.detail = None,
            Some(item) => {
                let picked = self.graph.node_of(item).map(|n| (n.id.clone(), matches!(n.kind, NodeKind::Note)));
                match picked {
                    Some((node, true)) => self.notes.request_open(&node, &self.tx),
                    Some((node, false)) => self.detail = Some(node),
                    None => {}
                }
            }
            None => {}
        }
        let (items, links) = self.graph.counts();
        let line = if self.graph.form == Form::Roots { format!("{items} itens") } else { format!("{items} itens · {links} ligações") };
        let hint = match self.graph.form {
            Form::Universe => "Arraste para girar · role para aproximar · estrela é memória, planeta é nota",
            _ => "Role para aproximar · arraste para mover · disco é memória, anel é nota",
        };
        // Com zoom o desenho passa por baixo destas linhas: um fundo mantém o texto legível.
        let st = ui_style(12.0).color(THEME.mute);
        let (left, right) = (ui.line(&line, st), ui.line(hint, st));
        let y = stage.bottom() - 28.0;
        ui.s.round(Rect::new(stage.x + 12.0, y - 5.0, left.width + 16.0, left.height + 10.0), 6.0, THEME.bg.alpha(0.86));
        ui.s.round(Rect::new(stage.right() - 28.0 - right.width, y - 5.0, right.width + 16.0, right.height + 10.0), 6.0, THEME.bg.alpha(0.86));
        ui.draw(&left, stage.x + 20.0, y);
        ui.draw(&right, stage.right() - 20.0 - right.width, y);
    }

    fn sidebar(&mut self, ui: &mut Ui<'_>, r: Rect) {
        ui.s.fill(r, THEME.side);
        ui.s.fill(Rect::new(r.right() - 1.0, r.y, 1.0, r.h), THEME.line);
        let x = r.x + 12.0;
        let w = r.w - 24.0;
        let mut y = 42.0;
        ui.mark(x + 18.0, y + 12.0, 20.0, THEME.ink);
        ui.text("Omnimind", ui_style(15.0).weight(600), x + 34.0, y + 2.0);
        y += 36.0;
        let search = Rect::new(x, y, w, 34.0);
        if ui.pressed(id("search"), search) {
            self.palette.show();
        }
        let hover = ui.hover(search) as u32 as f32;
        let a = ui.anim(id("search-hover"), hover, 18.0);
        ui.s.panel(search, 8.0, THEME.bg, THEME.line.mix(THEME.mute, a), 1.0);
        ui.text("Buscar", ui_style(13.5).color(THEME.mute), search.x + 12.0, search.y + 8.0);
        ui.text_right(&crate::platform::keys("⌘K"), mono(11.0), search.right() - 10.0, search.y + 10.0);
        y += 52.0;
        let at = NAV.iter().position(|n| n.0 == self.page);
        if let Some(i) = at {
            let py = ui.anim(id("nav-pill"), y + i as f32 * 36.0, 20.0);
            ui.s.round(Rect::new(x, py, w, 34.0), 8.0, THEME.sel);
        }
        let mut go = None;
        let live_memories = self.model.memories.iter().filter(|m| !matches!(m.state, MemState::Archived | MemState::Superseded)).count();
        for (i, (p, label)) in NAV.iter().enumerate() {
            let row = Rect::new(x, y + i as f32 * 36.0, w, 34.0);
            let on = *p == self.page;
            if ui.pressed(id(("nav", label)), row) {
                go = Some(*p);
            }
            let hover = ui.hover(row);
            let st = ui_style(14.0).weight(if on { 500 } else { 400 }).color(if on || hover { THEME.ink } else { THEME.sub });
            ui.text(label, st, row.x + 10.0, row.y + 8.0);
            let count = match p {
                Page::Notes => self.model.notes.len(),
                Page::Memories => live_memories,
                _ => 0,
            };
            if count > 0 {
                ui.text_right(&crate::views::metrics::thousands(count as u32), ui_style(12.0).color(THEME.mute), row.right() - 10.0, row.y + 9.0);
            }
        }
        if let Some(p) = go {
            self.go(p);
        }
        let foot = r.bottom() - 90.0;
        let active = self.live.as_ref().is_some_and(|l| self.time - l.1 < 6.0);
        if active {
            ui.animating = true;
        }
        // Conta projetos, não clientes: um projeto com os dois ainda é um.
        let mut on: Vec<&str> = self.model.links.iter().filter(|l| l.state == crate::model::LinkState::On).map(|l| l.project.as_str()).collect();
        on.sort_unstable();
        on.dedup();
        let on_links = on.len();
        let global = !self.model.global_links.is_empty();
        let status = match (&self.live, active) {
            (Some(l), true) => l.0.clone(),
            _ if global => "Ligado em todos os projetos".to_string(),
            _ => match on_links {
                0 => "Nenhum projeto ligado".to_string(),
                1 => "Ligado em 1 projeto".to_string(),
                n => format!("Ligado em {n} projetos"),
            },
        };
        let pulse = if active { 0.6 + 0.4 * (self.time * 5.0).sin() } else { 1.0 };
        let dot = if active || on_links > 0 || global { THEME.acc.alpha(pulse) } else { THEME.mute };
        ui.s.round(Rect::new(x + 10.0, foot + 15.0, 7.0, 7.0), 3.5, dot);
        let st = ui_style(12.5).color(THEME.sub);
        let t = ui.ellipsize(&status, st, w - 34.0);
        ui.text(&t, st, x + 24.0, foot + 9.0);
        let row = Rect::new(x, foot + 40.0, w, 34.0);
        let on = self.page == Page::Settings;
        if on {
            ui.s.round(row, 8.0, THEME.sel);
        }
        if ui.pressed(id("nav-settings"), row) {
            self.go(Page::Settings);
        }
        let hover = ui.hover(row);
        ui.text("Ajustes", ui_style(14.0).weight(if on { 500 } else { 400 }).color(if on || hover { THEME.ink } else { THEME.sub }), row.x + 10.0, row.y + 8.0);
    }

    fn title_drag(ui: &mut Ui<'_>, w: f32) -> bool {
        let bar = Rect::new(0.0, 0.0, w, 36.0);
        let (mx, my) = ui.st.mouse;
        ui.st.clicked && bar.contains(mx, my) && ui.st.hot.is_none() && !ui.covered()
    }

    fn toasts(&mut self, ui: &mut Ui<'_>, area: Rect) {
        if let Some((t, at)) = self.toast.clone() {
            let age = self.time - at;
            if age > 4.5 {
                self.toast = None;
                return;
            }
            ui.animating = true;
            let a = (age * 8.0).min(1.0) * (1.0 - (age - 3.8).max(0.0) / 0.7).clamp(0.0, 1.0);
            let st = ui_style(13.5).color(THEME.bg.alpha(a));
            let l = ui.line(&t, st);
            let r = Rect::new(area.x + (area.w - l.width - 28.0) * 0.5, area.bottom() - 64.0 + 10.0 * (1.0 - a), l.width + 28.0, 36.0);
            ui.s.round(r, 10.0, THEME.ink.alpha(a));
            ui.draw(&l, r.x + 14.0, r.y + 8.0);
        }
    }

    fn detail_panel(&mut self, ui: &mut Ui<'_>, main: Rect) {
        let m = self.model.clone();
        let mem: Option<&Memory> = self.detail.as_deref().and_then(|d| m.memories.iter().find(|x| x.id == d));
        let a = ui.anim(id("detail"), mem.is_some() as u32 as f32, 16.0);
        let Some(mem) = mem else {
            self.detail = None;
            return;
        };
        let r = Rect::new(main.right() - PANEL_W * a, 0.0, PANEL_W, main.h);
        ui.overlay(r);
        ui.in_overlay = true;
        ui.s.shadow(r, 0.0, 30.0, THEME.shadow);
        ui.s.fill(r, THEME.panel);
        ui.s.fill(Rect::new(r.x, r.y, 1.0, r.h), THEME.line);
        let (x, w) = (r.x + 24.0, PANEL_W - 48.0);
        let top = r.y + 44.0;
        ui.glyph(mem.kind as u8, x + 6.0, top + 9.0, 5.0, THEME.sub);
        ui.text(mem.kind.label(), ui_style(13.0).color(THEME.sub), x + 20.0, top);
        if ui.button(id("detail-close"), Rect::new(r.right() - 52.0, top - 7.0, 32.0, 32.0), "×") {
            self.detail = None;
            ui.in_overlay = false;
            return;
        }
        // O corpo vem sob demanda, e de novo se a memória mudar: a tela nunca mostra texto velho.
        let wanted = (mem.id.clone(), mem.hash.clone());
        if self.detail_asked != wanted {
            self.detail_asked = wanted;
            let _ = self.tx.send(Cmd::LoadMemory(mem.id.clone()));
        }
        let text = self.detail_body.as_ref().filter(|b| b.0 == mem.id && b.1 == mem.hash).map(|b| b.2.clone());
        // Rolagem no meio, botões fixos no rodapé: o fim de um texto longo sempre pode ser lido.
        let body = Rect::new(r.x, top + 34.0, PANEL_W, (r.h - 34.0 - 44.0 - 68.0).max(80.0));
        let off = ui.scroll(id(("detail-scroll", &mem.id)), body, self.detail_height);
        ui.s.push_clip(body);
        let start = body.y - off;
        let mut y = start;
        y += ui.para(&mem.title, ui_style(21.0).weight(600).leading(1.28), x, y, w) + 10.0;
        y += ui.para(&mem.summary, ui_style(14.5).color(THEME.sub).leading(1.6), x, y, w) + 12.0;
        if let Some(t) = text.as_deref().filter(|t| !t.is_empty()) {
            y += ui.para(t, ui_style(14.5).leading(1.6), x, y, w) + 14.0;
        }
        ui.s.fill(Rect::new(x, y, w, 1.0), THEME.line);
        y += 14.0;
        let source = if mem.client.is_empty() { mem.authority.label().to_string() } else { format!("{} · {}", mem.authority.label(), crate::stats::client_label(&mem.client)) };
        let note_title = mem.note.as_deref().map(|p| m.notes.iter().find(|n| n.path == p).map_or(p.to_string(), |n| n.title.clone()));
        let mut fields: Vec<(&str, String)> = vec![("Origem", source), ("Status", mem.state.label().to_string()), ("Criada", mem.created.clone())];
        if let Some(n) = note_title {
            fields.insert(2, ("Fonte", n));
        }
        if let Some(h) = &mem.held {
            fields.push(("Motivo", h.clone()));
        }
        for (k, v) in fields {
            ui.text(k, ui_style(13.5).color(THEME.mute), x, y);
            y += ui.para(&v, ui_style(13.5), x + 96.0, y, w - 96.0).max(20.0) + 6.0;
        }
        // De que projeto é: só a pessoa muda, e "geral" faz a memória valer em todo lugar.
        ui.text("Projeto", ui_style(13.5).color(THEME.mute), x, y);
        let project = mem.project.as_deref().map(|p| m.projects.iter().find(|i| i.slug == p).map_or(p.to_string(), |i| i.title.clone()));
        let shown = project.clone().unwrap_or_else(|| "Geral, vale em todo projeto".into());
        let (change, lw) = ui.link(id(("detail-project", &mem.id)), x + w - 44.0, y, "Mudar", ui_style(13.5).color(THEME.acc));
        let _ = lw;
        y += ui.para(&shown, ui_style(13.5), x + 96.0, y, w - 96.0 - 56.0).max(20.0) + 6.0;
        if change {
            let options = self.project_options(mem.project.as_deref());
            self.palette.choose(Choice { kind: ChoiceKind::MemoryProject { id: mem.id.clone(), hash: mem.hash.clone() }, label: "Projeto desta memória", options, free_text: false });
        }
        if !mem.quote.is_empty() {
            y += 8.0;
            ui.s.fill(Rect::new(x, y, w, 1.0), THEME.line);
            y += 14.0;
            y += ui.para(&format!("“{}”", mem.quote), ui_style(14.0).italic().color(THEME.sub).leading(1.55), x, y, w) + 8.0;
        }
        ui.s.pop_clip();
        self.detail_height = y - start + 12.0;
        let foot = Rect::new(r.x, body.bottom(), PANEL_W, 68.0);
        ui.s.fill(Rect::new(foot.x, foot.y, foot.w, 1.0), THEME.line);
        let by = foot.y + 16.0;
        let mut bx = x;
        // Aprovar exige ter visto o texto atual, com o invisível à mostra.
        if mem.state == MemState::Quarantined && text.is_some() {
            if ui.primary(id("approve"), Rect::new(bx, by, 92.0, 34.0), "Aprovar") {
                let _ = self.tx.send(Cmd::Promote { id: mem.id.clone(), hash: mem.hash.clone() });
                self.detail = None;
            }
            bx += 100.0;
        }
        if let Some(n) = mem.note.clone() {
            if ui.button(id("open-origin"), Rect::new(bx, by, 104.0, 34.0), "Abrir a nota") {
                self.notes.request_open(&n, &self.tx);
            }
            bx += 112.0;
        }
        if ui.button(id("archive"), Rect::new(bx, by, 92.0, 34.0), "Arquivar") {
            let _ = self.tx.send(Cmd::Forget(mem.id.clone()));
            self.detail = None;
        }
        ui.in_overlay = false;
    }

    fn welcome(&mut self, ui: &mut Ui<'_>, area: Rect) {
        let a = ui.anim_from(id("welcome"), 0.0, 1.0, 3.0);
        let left = 560f32.min(area.w * 0.46);
        let art = area.pad(left, 0.0, 0.0, 0.0);
        self.graph.paint(ui, art, self.image, false);
        let (x, w) = (88.0, left - 112.0);
        let mut y = area.h * 0.5 - 232.0 + 8.0 * (1.0 - a);
        ui.mark(x + 28.0, y + 28.0, 56.0, THEME.ink.alpha(a));
        y += 76.0;
        ui.text("Omnimind", ui_style(52.0).weight(600).leading(1.1).color(THEME.ink.alpha(a)), x, y);
        y += 76.0;
        y += ui.para("Suas notas e o que a IA aprende com você, no mesmo lugar.", ui_style(22.0).leading(1.32).color(THEME.ink.alpha(a)), x, y, w) + 16.0;
        y += ui.para("Tudo fica em arquivos seus, nesta máquina. A IA grava sozinha, e você vê, entende e desfaz o que ela gravou.", ui_style(16.0).leading(1.6).color(THEME.sub.alpha(a)), x, y, w) + 18.0;
        for (k, v) in [("Nota", "O que você escreve."), ("Memória", "O que a IA aprende."), ("Cofre", "A pasta que guarda as duas.")] {
            ui.text(k, ui_style(14.5).weight(500).color(THEME.ink.alpha(a)), x, y);
            ui.text(v, ui_style(14.5).color(THEME.sub.alpha(a)), x + 84.0, y);
            y += 26.0;
        }
        y += 20.0;
        let b = Rect::new(x, y, 130.0, 44.0);
        self.start_button = b;
        if ui.primary(id("w-new"), b, "Começar") {
            let _ = self.tx.send(Cmd::CreateBrain(crate::default_brain_path()));
        }
        let link = ui_style(14.5).color(THEME.sub);
        let (open, w1) = ui.link(id("w-open"), x + 152.0, y + 12.0, "Abrir uma pasta", link);
        if open {
            if let Some(p) = crate::pick_folder("Escolha a pasta do cofre") {
                let _ = self.tx.send(Cmd::OpenBrain(p));
            }
        }
        let (demo, _) = ui.link(id("w-demo"), x + 176.0 + w1, y + 12.0, "Ver um exemplo", link);
        if demo {
            let _ = self.tx.send(Cmd::OpenBrain("example".into()));
        }
    }
}

fn help_sheet(ui: &mut Ui<'_>, area: Rect) {
    const KEYS: [(&str, &str); 10] = [
        ("⌘K", "Buscar em tudo e rodar comandos"),
        ("⌘1 a ⌘6", "Notas, Memórias, Grafo, Atividade, Métricas, Perfil"),
        ("⌘,", "Ajustes"),
        ("⌘N", "Nova nota"),
        ("⌘D", "Nota do dia"),
        ("⌘[  ⌘]", "Voltar e avançar entre notas"),
        ("[[", "Criar link, com sugestões"),
        ("⌘ + clique", "Seguir um link"),
        ("⌘B  ⌘I", "Negrito e itálico"),
        ("Esc", "Fechar o painel"),
    ];
    ui.s.fill(area, THEME.scrim);
    let (w, h) = (560.0, 40.0 + KEYS.len() as f32 * 30.0 + 26.0);
    let r = Rect::new((area.w - w) * 0.5, (area.h - h) * 0.5, w, h);
    ui.overlay(area);
    ui.s.shadow(r, 14.0, 34.0, THEME.shadow);
    ui.s.panel(r, 14.0, THEME.panel, THEME.line, 1.0);
    ui.text("Atalhos", ui_style(15.0).weight(600), r.x + 26.0, r.y + 20.0);
    for (i, (k, t)) in KEYS.iter().enumerate() {
        let y = r.y + 54.0 + i as f32 * 30.0;
        ui.text(&crate::platform::keys(k), mono(12.5), r.x + 26.0, y);
        ui.text(t, ui_style(14.0).color(THEME.sub), r.x + 150.0, y - 1.0);
    }
}

/// Fundo das boas-vindas: bonito sem depender de dado nenhum.
fn demo_graph() -> Graph {
    let mut nodes = Vec::new();
    for i in 0..300u32 {
        let kind = if i % 3 == 0 { NodeKind::Note } else { NodeKind::Memory(MemKind::Decision) };
        nodes.push(GraphNode { id: format!("d{i}"), kind, title: String::new(), weight: (i % 7) as f32, region: i % 7, project: u32::MAX, hub: false });
    }
    Graph { nodes, edges: vec![], regions: ["1", "2", "3", "4", "5", "6", "7"].map(|_| String::new()).to_vec(), projects: vec![] }
}
