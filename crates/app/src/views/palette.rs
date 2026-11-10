//! ⌘K: procura em tudo e leva a qualquer lugar. Responde enquanto se digita.

use crate::command::{Cmd, Hit};
use crate::theme::{mono, ui as ui_style, THEME};
use crate::ui::{id, Ui};
use ed::Buffer;
use gfx::Rect;
use std::sync::mpsc::Sender;

pub struct Palette {
    pub open: bool,
    pub buf: Buffer,
    pub hits: Vec<Hit>,
    pub note: String,
    pub cursor: usize,
    seq: u64,
    sent: String,
    pub prompt: Option<(Prompt, &'static str)>,
    /// Lista fechada de opções, no lugar da busca: escolher o projeto de uma memória, por exemplo.
    pub choice: Option<Choice>,
}

pub enum Picked {
    Hit(Hit),
    Action(Action),
    Prompted(Prompt, String),
    Chosen(ChoiceKind, Option<String>),
}

/// Para que serve a escolha em curso.
#[derive(Clone, Debug, PartialEq)]
pub enum ChoiceKind {
    MemoryProject { id: String, hash: String },
    MoveProject { from: String },
    ProjectGroup { slug: String },
}

#[derive(Clone, Debug)]
pub struct ChoiceOption {
    pub label: String,
    pub detail: String,
    pub value: Option<String>,
}

pub struct Choice {
    pub kind: ChoiceKind,
    pub label: &'static str,
    pub options: Vec<ChoiceOption>,
    /// Aceita o que a pessoa digitar como opção nova, como o nome de um grupo.
    pub free_text: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Page(usize),
    Settings,
    NewNote,
    Daily,
    Rename,
    Delete,
    OpenBrain,
    Help,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Prompt {
    NewNote,
    Rename,
}

const COMMANDS: [(&str, &str, Action); 13] = [
    ("Ir para Notas", "⌘1", Action::Page(0)),
    ("Ir para Memórias", "⌘2", Action::Page(1)),
    ("Ir para Grafo", "⌘3", Action::Page(2)),
    ("Ir para Atividade", "⌘4", Action::Page(3)),
    ("Ir para Métricas", "⌘5", Action::Page(4)),
    ("Ir para Perfil", "⌘6", Action::Page(5)),
    ("Ir para Ajustes", "⌘,", Action::Settings),
    ("Nova nota", "⌘N", Action::NewNote),
    ("Nota do dia", "⌘D", Action::Daily),
    ("Renomear nota", "", Action::Rename),
    ("Apagar nota", "", Action::Delete),
    ("Abrir outro cofre", "", Action::OpenBrain),
    ("Atalhos", "⌘/", Action::Help),
];

impl Palette {
    pub fn new() -> Self {
        Self { open: false, buf: Buffer::new(""), hits: vec![], note: String::new(), cursor: 0, seq: 0, sent: String::new(), prompt: None, choice: None }
    }

    pub fn show(&mut self) {
        self.open = true;
        self.buf = Buffer::new("");
        self.hits.clear();
        self.note.clear();
        self.cursor = 0;
        self.sent.clear();
        self.prompt = None;
        self.choice = None;
    }

    /// Abre a paleta com uma lista fechada de opções. Digitar filtra.
    pub fn choose(&mut self, c: Choice) {
        self.show();
        self.choice = Some(c);
    }

    /// Opções que combinam com o que foi digitado. Com texto livre, o digitado entra como primeira.
    fn options(&self) -> Vec<ChoiceOption> {
        let Some(c) = &self.choice else { return vec![] };
        let typed = self.buf.text().trim().to_string();
        let q = ed::fold(&typed);
        let mut list: Vec<ChoiceOption> = c.options.iter().filter(|o| q.is_empty() || ed::fold(&o.label).contains(&q)).cloned().collect();
        if c.free_text && !typed.is_empty() && !c.options.iter().any(|o| ed::fold(&o.label) == q) {
            list.insert(0, ChoiceOption { label: typed.clone(), detail: "novo".into(), value: Some(typed) });
        }
        list.truncate(12);
        list
    }

    /// Reaproveita o campo para pedir um texto, como o nome de uma nota.
    pub fn ask(&mut self, p: Prompt, label: &'static str, initial: &str) {
        self.show();
        self.buf = Buffer::new(initial);
        let n = self.buf.len_chars();
        self.buf.set_caret(n, false);
        self.prompt = Some((p, label));
    }

    fn commands(&self) -> Vec<&'static (&'static str, &'static str, Action)> {
        let q = ed::fold(&self.buf.text());
        COMMANDS.iter().filter(|c| q.is_empty() || ed::fold(c.0).contains(&q)).collect()
    }

    pub fn results(&mut self, seq: u64, hits: Vec<Hit>, note: String) {
        if seq == self.seq {
            self.hits = hits;
            self.note = note;
            self.cursor = 0;
        }
    }

    fn query_changed(&mut self, tx: &Sender<Cmd>) {
        let q = self.buf.text();
        if q != self.sent && self.prompt.is_none() && self.choice.is_none() {
            self.sent = q.clone();
            self.seq += 1;
            let _ = tx.send(Cmd::Search { query: q, seq: self.seq });
        }
    }

    fn row_count(&self) -> usize {
        if self.choice.is_some() {
            return self.options().len();
        }
        if self.prompt.is_some() { 0 } else { self.commands().len().min(if self.buf.len_chars() == 0 { COMMANDS.len() } else { 3 }) + self.hits.len().min(8) }
    }

    pub fn move_cursor(&mut self, d: i32) {
        let n = self.row_count() as i32;
        if n > 0 {
            self.cursor = (self.cursor as i32 + d).rem_euclid(n) as usize;
        }
    }

    pub fn pick(&self) -> Option<Picked> {
        if let Some(c) = &self.choice {
            return self.options().get(self.cursor).map(|o| Picked::Chosen(c.kind.clone(), o.value.clone()));
        }
        if let Some((p, _)) = self.prompt {
            return Some(Picked::Prompted(p, self.buf.text()));
        }
        let cmds = self.commands();
        let shown = cmds.len().min(if self.buf.len_chars() == 0 { COMMANDS.len() } else { 3 });
        if self.cursor < shown {
            return Some(Picked::Action(cmds[self.cursor].2));
        }
        self.hits.get(self.cursor - shown).cloned().map(Picked::Hit)
    }

    pub fn paint(&mut self, ui: &mut Ui<'_>, area: Rect, tx: &Sender<Cmd>) -> Option<Picked> {
        self.query_changed(tx);
        let a = ui.anim_from(id("palette-in"), 0.0, 1.0, 26.0);
        ui.s.fill(area, THEME.scrim.alpha(THEME.scrim.a * a));
        let w = 580f32.min(area.w - 60.0);
        if self.choice.is_some() {
            return self.paint_choice(ui, area, w, a);
        }
        let cmds = self.commands();
        let empty = self.buf.len_chars() == 0;
        let shown = cmds.len().min(if empty { COMMANDS.len() } else { 3 });
        let hits = self.hits.clone();
        let rows = shown as f32 * 38.0 + hits.len().min(8) as f32 * 50.0;
        let nothing = shown == 0 && hits.is_empty() && self.prompt.is_none();
        let h = 62.0 + rows + if nothing { 46.0 } else { 6.0 };
        let r = Rect::new(area.x + (area.w - w) * 0.5, area.y + 80.0 - 10.0 * (1.0 - a), w, h);
        ui.overlay(area);
        ui.in_overlay = true;
        ui.s.shadow(r, 14.0, 34.0, THEME.shadow);
        ui.s.panel(r, 14.0, THEME.panel, THEME.line, 1.0);
        let text = self.buf.text();
        let st = ui_style(16.0);
        let placeholder = self.prompt.map_or("Buscar notas, memórias e comandos", |p| p.1);
        let shown_text = if text.is_empty() { placeholder } else { text.as_str() };
        let l = ui.line(shown_text, if text.is_empty() { st.color(THEME.mute) } else { st });
        ui.draw(&l, r.x + 18.0, r.y + 17.0);
        let cx = if text.is_empty() { -1.0 } else { l.caret(self.buf.line_byte(self.buf.head()).1.min(text.len())).0 };
        ui.s.fill(Rect::new(r.x + 18.0 + cx, r.y + 17.0, 1.5, 22.0), THEME.acc);
        ui.s.fill(Rect::new(r.x, r.y + 56.0, r.w, 1.0), THEME.line);
        let mut y = r.y + 60.0;
        let mut picked = None;
        for (i, c) in cmds.iter().take(shown).enumerate() {
            let row = Rect::new(r.x + 6.0, y, r.w - 12.0, 36.0);
            if ui.pressed(id(("pal-cmd", i, c.0)), row) {
                picked = Some(Picked::Action(c.2));
            }
            if self.cursor == i || ui.hover(row) {
                ui.s.round(row, 8.0, THEME.hover);
            }
            ui.text(c.0, ui_style(14.0), row.x + 12.0, y + 9.0);
            ui.text_right(&crate::platform::keys(c.1), mono(11.0), row.right() - 12.0, y + 11.0);
            y += 38.0;
        }
        if nothing {
            let n = if self.note.is_empty() { "Nada encontrado.".to_string() } else { self.note.clone() };
            ui.text(&n, ui_style(14.0).color(THEME.sub), r.x + 20.0, y + 12.0);
        }
        for (i, hit) in hits.iter().take(8).enumerate() {
            let row = Rect::new(r.x + 6.0, y, r.w - 12.0, 46.0);
            if ui.pressed(id(("pal-hit", i)), row) {
                picked = Some(Picked::Hit(hit.clone()));
            }
            if self.cursor == i + shown || ui.hover(row) {
                ui.s.round(row, 8.0, THEME.hover);
            }
            ui.glyph(if hit.is_note { 0 } else { hit.kind }, row.x + 16.0, y + 15.0, 5.0, THEME.acc);
            let st = ui_style(14.5).weight(600);
            let t = ui.ellipsize(&hit.title, st, row.w - 50.0);
            ui.text(&t, st, row.x + 32.0, y + 6.0);
            let st2 = ui_style(12.5).color(THEME.sub);
            let s = ui.ellipsize(&hit.snippet, st2, row.w - 50.0);
            ui.text(&s, st2, row.x + 32.0, y + 25.0);
            y += 50.0;
        }
        ui.in_overlay = false;
        picked
    }
}

impl Palette {
    /// A mesma folha da busca, com uma lista de opções no lugar dos resultados.
    fn paint_choice(&mut self, ui: &mut Ui<'_>, area: Rect, w: f32, a: f32) -> Option<Picked> {
        let options = self.options();
        self.cursor = self.cursor.min(options.len().saturating_sub(1));
        let (kind, label) = self.choice.as_ref().map(|c| (c.kind.clone(), c.label))?;
        let h = 62.0 + options.len() as f32 * 38.0 + if options.is_empty() { 46.0 } else { 6.0 };
        let r = Rect::new(area.x + (area.w - w) * 0.5, area.y + 80.0 - 10.0 * (1.0 - a), w, h);
        ui.overlay(area);
        ui.in_overlay = true;
        ui.s.shadow(r, 14.0, 34.0, THEME.shadow);
        ui.s.panel(r, 14.0, THEME.panel, THEME.line, 1.0);
        let text = self.buf.text();
        let st = ui_style(16.0);
        let l = ui.line(if text.is_empty() { label } else { text.as_str() }, if text.is_empty() { st.color(THEME.mute) } else { st });
        ui.draw(&l, r.x + 18.0, r.y + 17.0);
        let cx = if text.is_empty() { -1.0 } else { l.caret(self.buf.line_byte(self.buf.head()).1.min(text.len())).0 };
        ui.s.fill(Rect::new(r.x + 18.0 + cx, r.y + 17.0, 1.5, 22.0), THEME.acc);
        ui.s.fill(Rect::new(r.x, r.y + 56.0, r.w, 1.0), THEME.line);
        let mut y = r.y + 60.0;
        let mut picked = None;
        if options.is_empty() {
            ui.text("Nada com esse nome.", ui_style(14.0).color(THEME.sub), r.x + 20.0, y + 12.0);
        }
        for (i, o) in options.iter().enumerate() {
            let row = Rect::new(r.x + 6.0, y, r.w - 12.0, 36.0);
            if ui.pressed(id(("pal-choice", i)), row) {
                picked = Some(Picked::Chosen(kind.clone(), o.value.clone()));
            }
            if self.cursor == i || ui.hover(row) {
                ui.s.round(row, 8.0, THEME.hover);
            }
            let detail = ui.text_right(&o.detail, ui_style(12.5).color(THEME.mute), row.right() - 12.0, y + 10.0);
            let st = ui_style(14.0);
            let t = ui.ellipsize(&o.label, st, row.w - detail - 40.0);
            ui.text(&t, st, row.x + 12.0, y + 9.0);
            y += 38.0;
        }
        ui.in_overlay = false;
        picked
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}
