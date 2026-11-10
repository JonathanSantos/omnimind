//! Notas: lista à esquerda, editor no meio, e à direita o que a IA tirou dali.

use super::editor::EditorView;
use crate::command::Cmd;
use crate::input::{Event, Key, Mods};
use crate::model::{Memory, Model};
use crate::theme::{mono, ui as ui_style, THEME};
use crate::ui::{id, Ui};
use ed::Move;
use gfx::Rect;
use std::collections::HashSet;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

const AUTOSAVE_IDLE: Duration = Duration::from_millis(350);
const AUTOSAVE_MAX: Duration = Duration::from_secs(2);
const RETRY: Duration = Duration::from_secs(2);

/// Estado de gravação do que está aberto. A regra: nada digitado some sem escolha.
#[derive(Clone, Debug, PartialEq)]
pub enum SaveState {
    Clean,
    Dirty,
    Saving,
    /// O disco mudou enquanto havia texto sem salvar. A pessoa decide.
    Conflict { disk: String, hash: String },
}

/// O que a tela de notas pede à casca.
#[derive(Default)]
pub struct NotesOut {
    pub new_note: bool,
    pub open_memory: Option<String>,
}

pub struct NotesView {
    pub editor: Option<EditorView>,
    pub path: Option<String>,
    pub base: String,
    pub save: SaveState,
    saved_version: u64,
    first_dirty: Option<Instant>,
    last_edit: Option<Instant>,
    pending_open: Option<String>,
    drag: bool,
    pending_history: bool,
    history: Vec<String>,
    forward: Vec<String>,
    /// Sugestões de `[[` abertas e qual está marcada.
    complete: Vec<String>,
    complete_at: usize,
    pub complete_open: bool,
    /// Pediram outra nota durante um conflito: a casca avisa em vez de ficar muda.
    pub blocked: bool,
    view_h: f32,
    link_seen: (u64, u64, usize),
}

impl NotesView {
    pub fn new() -> Self {
        Self {
            editor: None,
            path: None,
            base: String::new(),
            save: SaveState::Clean,
            saved_version: 0,
            first_dirty: None,
            last_edit: None,
            pending_open: None,
            drag: false,
            pending_history: false,
            history: vec![],
            forward: vec![],
            complete: vec![],
            complete_at: 0,
            complete_open: false,
            blocked: false,
            view_h: 800.0,
            link_seen: (u64::MAX, 0, 0),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.editor.as_ref().is_some_and(|e| e.buf.version != self.saved_version)
    }

    /// Trocar de nota nunca abandona texto: grava primeiro, abre depois de confirmar.
    pub fn request_open(&mut self, path: &str, tx: &Sender<Cmd>) {
        if self.path.as_deref() == Some(path) {
            return;
        }
        if matches!(self.save, SaveState::Conflict { .. }) {
            self.blocked = true;
            return;
        }
        if self.is_dirty() {
            self.flush(tx);
            self.pending_open = Some(path.to_string());
        } else {
            let _ = tx.send(Cmd::OpenNote(path.to_string()));
        }
    }

    pub fn back(&mut self, tx: &Sender<Cmd>) {
        if let Some(p) = self.history.pop() {
            if let Some(cur) = self.path.clone() {
                self.forward.push(cur);
            }
            self.pending_history = true;
            self.request_open(&p, tx);
        }
    }

    pub fn ahead(&mut self, tx: &Sender<Cmd>) {
        if let Some(p) = self.forward.pop() {
            if let Some(cur) = self.path.clone() {
                self.history.push(cur);
            }
            self.pending_history = true;
            self.request_open(&p, tx);
        }
    }

    /// Toda nota aberta passa por aqui. O que estava no editor nunca some sem chegar ao disco.
    pub fn opened(&mut self, path: String, text: String, hash: String, tx: &Sender<Cmd>) {
        if self.path.as_deref() != Some(path.as_str()) {
            if let (Some(e), Some(p)) = (&self.editor, &self.path) {
                if matches!(self.save, SaveState::Conflict { .. }) {
                    let _ = tx.send(Cmd::KeepBoth { path: p.clone(), text: e.buf.text() });
                } else if self.is_dirty() {
                    let _ = tx.send(Cmd::SaveLeaving { path: p.clone(), text: e.buf.text(), base: self.base.clone() });
                }
            }
        }
        if !std::mem::take(&mut self.pending_history) {
            if let Some(cur) = self.path.clone() {
                if cur != path {
                    self.history.push(cur);
                    self.forward.clear();
                }
            }
        }
        self.editor = Some(EditorView::new(&text));
        self.saved_version = 0;
        self.path = Some(path);
        self.base = hash;
        self.save = SaveState::Clean;
        self.first_dirty = None;
        self.last_edit = None;
    }

    pub fn saved(&mut self, path: &str, hash: String, tx: &Sender<Cmd>) {
        if self.path.as_deref() == Some(path) {
            self.base = hash;
            if self.save == SaveState::Saving {
                self.save = if self.is_dirty() { SaveState::Dirty } else { SaveState::Clean };
            }
        }
        if !self.is_dirty() {
            if let Some(p) = self.pending_open.take() {
                let _ = tx.send(Cmd::OpenNote(p));
            }
        }
    }

    /// Disco cheio, pasta sumida: o texto continua aqui, marcado como não salvo.
    pub fn save_failed(&mut self, path: &str) {
        if self.path.as_deref() == Some(path) {
            self.saved_version = u64::MAX;
            self.save = SaveState::Dirty;
            self.last_edit = Some(Instant::now() + RETRY);
            self.pending_open = None;
        }
    }

    /// Último ato antes de sair. Em conflito, a versão daqui vai para uma nota irmã.
    pub fn flush_for_exit(&mut self, tx: &Sender<Cmd>) {
        let _ = tx.send(Cmd::Closing);
        if let (SaveState::Conflict { .. }, Some(e), Some(p)) = (&self.save, &self.editor, &self.path) {
            let _ = tx.send(Cmd::KeepBoth { path: p.clone(), text: e.buf.text() });
            return;
        }
        self.flush(tx);
    }

    pub fn conflict(&mut self, path: &str, disk: String, hash: String) {
        if self.path.as_deref() == Some(path) {
            self.save = SaveState::Conflict { disk, hash };
            self.pending_open = None;
        }
    }

    /// Mudança externa: sem edição pendente recarrega; com edição vira conflito.
    pub fn external(&mut self, path: &str, text: String, hash: String) {
        if self.path.as_deref() != Some(path) || self.base == hash {
            return;
        }
        // Gravação em voo conta como texto pendente: recarregar aqui perderia o que está indo.
        // Segundo evento durante um conflito só atualiza o que veio de fora.
        let conflict = matches!(self.save, SaveState::Conflict { .. });
        if self.is_dirty() || self.save == SaveState::Saving || conflict {
            self.save = SaveState::Conflict { disk: text, hash };
            return;
        }
        if let Some(e) = &mut self.editor {
            e.buf.reload(&text);
            self.saved_version = e.buf.version;
        }
        self.base = hash;
        self.save = SaveState::Clean;
    }

    pub fn flush(&mut self, tx: &Sender<Cmd>) {
        if !self.is_dirty() || matches!(self.save, SaveState::Conflict { .. }) {
            return;
        }
        if let (Some(e), Some(p)) = (&self.editor, &self.path) {
            let text = e.buf.text();
            self.saved_version = e.buf.version;
            self.save = SaveState::Saving;
            self.first_dirty = None;
            let _ = tx.send(Cmd::SaveNote { path: p.clone(), text, base: self.base.clone() });
        }
    }

    pub fn tick(&mut self, tx: &Sender<Cmd>) -> Option<Duration> {
        if !self.is_dirty() || matches!(self.save, SaveState::Conflict { .. }) {
            return None;
        }
        let now = Instant::now();
        let idle = self.last_edit.map_or(AUTOSAVE_IDLE, |t| now.saturating_duration_since(t));
        let held = self.first_dirty.map_or(Duration::ZERO, |t| now - t);
        if idle >= AUTOSAVE_IDLE || held >= AUTOSAVE_MAX {
            self.flush(tx);
            return None;
        }
        Some(AUTOSAVE_IDLE - idle)
    }

    pub fn resolve(&mut self, choice: u8, tx: &Sender<Cmd>) {
        let SaveState::Conflict { disk, hash } = self.save.clone() else { return };
        let Some(path) = self.path.clone() else { return };
        match choice {
            // Manter a minha: regrava por cima do que chegou, com o hash novo.
            0 => {
                self.base = hash;
                self.save = SaveState::Dirty;
                if let Some(e) = &self.editor {
                    self.saved_version = u64::MAX;
                    let _ = e;
                }
                self.flush(tx);
            }
            // Usar a do disco: o texto daqui vai para uma nota irmã antes de sair da tela.
            1 => {
                if let Some(e) = &mut self.editor {
                    let mine = e.buf.text();
                    let _ = tx.send(Cmd::KeepBoth { path: path.clone(), text: mine });
                    e.buf.reload(&disk);
                    self.saved_version = e.buf.version;
                }
                self.base = hash;
                self.save = SaveState::Clean;
            }
            _ => {}
        }
    }

    fn edited(&mut self) {
        let now = Instant::now();
        self.last_edit = Some(now);
        self.first_dirty.get_or_insert(now);
        if self.save == SaveState::Clean {
            self.save = SaveState::Dirty;
        }
    }

    /// Devolve true se o evento foi consumido pelo editor.
    pub fn event(&mut self, e: &Event, clipboard: &mut crate::Clipboard) -> bool {
        let view_h = self.view_h;
        if self.complete_open {
            if let Event::Key(k, _) = e {
                match k {
                    Key::Up => {
                        self.complete_at = self.complete_at.saturating_sub(1);
                        return true;
                    }
                    Key::Down => {
                        self.complete_at = (self.complete_at + 1).min(self.complete.len().saturating_sub(1));
                        return true;
                    }
                    Key::Enter | Key::Tab => {
                        if let (Some(t), Some(ed)) = (self.complete.get(self.complete_at).cloned(), self.editor.as_mut()) {
                            ed.complete_link(&t);
                            self.complete_open = false;
                            self.edited();
                            return true;
                        }
                    }
                    Key::Escape => {
                        self.complete_open = false;
                        return true;
                    }
                    _ => {}
                }
            }
        }
        let Some(ed) = self.editor.as_mut() else { return false };
        let before = ed.buf.version;
        match e {
            Event::Text(s) => {
                ed.buf.preedit.clear();
                ed.buf.insert(s);
            }
            Event::Preedit(s) => ed.buf.preedit = s.clone(),
            Event::Key(k, m) => {
                if !edit_key(ed, *k, *m, view_h, clipboard) {
                    return false;
                }
            }
            _ => return false,
        }
        ed.reveal_caret(view_h);
        if ed.buf.version != before {
            self.edited();
        }
        true
    }

    pub fn paint(&mut self, ui: &mut Ui<'_>, area: Rect, model: &Model, tx: &Sender<Cmd>) -> NotesOut {
        let mut out = NotesOut::default();
        let mut r = area;
        let list = r.take_left(300.0);
        let margin = if r.w > 860.0 { r.take_right(280.0) } else { Rect::default() };
        self.view_h = r.h - 30.0;
        ui.s.fill(Rect::new(list.right() - 1.0, list.y, 1.0, list.h), THEME.line);
        self.list(ui, list, model, tx, &mut out);
        self.update_missing(model);
        self.center(ui, r, model, tx);
        if margin.w > 0.0 {
            self.margin(ui, margin, model, tx, &mut out);
        }
        self.completion(ui, model);
        out
    }

    fn update_missing(&mut self, model: &Model) {
        let Some(ed) = self.editor.as_mut() else { return };
        if self.link_seen == (ed.buf.version, model.generation, model.notes.len()) {
            return;
        }
        self.link_seen = (ed.buf.version, model.generation, model.notes.len());
        let known: HashSet<String> = model.notes.iter().flat_map(|n| std::iter::once(n.title.to_lowercase()).chain(n.aliases.iter().map(|a| a.to_lowercase())).chain(std::iter::once(stem(&n.path).to_lowercase()))).collect();
        ed.missing.clear();
        for i in 0..ed.buf.len_lines() {
            let line = ed.buf.line(i);
            if !line.contains("[[") {
                continue;
            }
            for inl in ed::md::inlines(&line) {
                if inl.mark == ed::md::Mark::Wiki {
                    if let Some(t) = inl.target {
                        let t = line[t].to_lowercase();
                        if !known.contains(&t) {
                            ed.missing.insert(t);
                        }
                    }
                }
            }
        }
    }

    fn completion(&mut self, ui: &mut Ui<'_>, model: &Model) {
        let Some(ed) = self.editor.as_ref() else { return };
        let Some(q) = ed.open_link_query() else {
            self.complete_open = false;
            return;
        };
        let (cx, cy, ch) = ed.caret_screen;
        let fold = |s: &str| ed::fold(s);
        let want = fold(&q);
        let mut found: Vec<(usize, &str)> = model
            .notes
            .iter()
            .filter_map(|n| {
                let t = fold(&n.title);
                let at = t.find(&want).or_else(|| n.aliases.iter().find_map(|a| fold(a).find(&want).map(|p| p + 40)))?;
                Some((at, n.title.as_str()))
            })
            .collect();
        found.sort_by_key(|x| x.0);
        self.complete = found.into_iter().take(6).map(|x| x.1.to_string()).collect();
        if !q.trim().is_empty() && !self.complete.iter().any(|t| fold(t) == want) {
            self.complete.push(q.trim().to_string());
        }
        self.complete_open = !self.complete.is_empty();
        if !self.complete_open {
            return;
        }
        self.complete_at = self.complete_at.min(self.complete.len() - 1);
        let w = 320.0;
        let h = self.complete.len() as f32 * 30.0 + 8.0;
        let r = Rect::new(cx, cy + ch + 6.0, w, h);
        ui.overlay(r);
        ui.s.shadow(r, 10.0, 22.0, THEME.shadow);
        ui.s.panel(r, 10.0, THEME.panel, THEME.line, 1.0);
        for i in 0..self.complete.len() {
            let row = Rect::new(r.x + 4.0, r.y + 4.0 + i as f32 * 30.0, w - 8.0, 30.0);
            if i == self.complete_at {
                ui.s.round(row, 7.0, THEME.hover);
            }
            let exists = model.notes.iter().any(|n| n.title == self.complete[i]);
            let t = if exists { self.complete[i].clone() } else { format!("Criar “{}”", self.complete[i]) };
            ui.text(&t, ui_style(13.5), row.x + 10.0, row.y + 6.0);
        }
    }

    fn list(&mut self, ui: &mut Ui<'_>, r: Rect, model: &Model, tx: &Sender<Cmd>, out: &mut NotesOut) {
        ui.head(r.x + 20.0, r.y, "Notas");
        if ui.button(id("new-note"), Rect::new(r.right() - 48.0, r.y + 12.0, 32.0, 32.0), "+") {
            out.new_note = true;
        }
        let list = r.pad(0.0, 56.0, 1.0, 0.0);
        const ROW: f32 = 58.0;
        let off = ui.scroll(id("notes-list"), list, model.notes.len() as f32 * ROW);
        ui.s.push_clip(list);
        let first = (off / ROW) as usize;
        let last = ((off + list.h) / ROW) as usize + 1;
        for (row, n) in model.notes.iter().enumerate().take(last.min(model.notes.len())).skip(first) {
            let y = list.y + row as f32 * ROW - off;
            let rr = Rect::new(list.x, y, list.w, ROW);
            let active = self.path.as_deref() == Some(n.path.as_str());
            let key = id(("note", &n.path));
            if ui.pressed(key, rr) {
                self.request_open(&n.path, tx);
            }
            let hover = ui.hover(rr);
            let a = ui.anim(key ^ 3, (hover || active) as u32 as f32, 22.0);
            if a > 0.01 {
                ui.s.fill(rr, (if active { THEME.sel } else { THEME.hover }).alpha(a));
            }
            ui.s.fill(Rect::new(rr.x, y, rr.w, 1.0), THEME.line);
            let st = ui_style(14.0).weight(500);
            let title = ui.ellipsize(&n.title, st, rr.w - 40.0);
            ui.text(&title, st, rr.x + 20.0, y + 11.0);
            let sub = match (n.folder.is_empty(), n.memories) {
                (true, 0) => n.modified.clone(),
                (true, m) => format!("{} · {m} {}", n.modified, if m == 1 { "memória" } else { "memórias" }),
                (false, 0) => format!("{} · {}", n.folder, n.modified),
                (false, m) => format!("{} · {} · {m} {}", n.folder, n.modified, if m == 1 { "memória" } else { "memórias" }),
            };
            let st2 = ui_style(12.5).color(THEME.mute);
            let sub = ui.ellipsize(&sub, st2, rr.w - 40.0);
            ui.text(&sub, st2, rr.x + 20.0, y + 32.0);
        }
        ui.s.pop_clip();
    }

    fn center(&mut self, ui: &mut Ui<'_>, mut r: Rect, model: &Model, tx: &Sender<Cmd>) {
        let bar = r.take_top(44.0);
        let Some(path) = self.path.clone() else {
            let y = r.y + r.h * 0.38;
            ui.text_center(&crate::platform::keys("Escolha uma nota, ou crie uma com ⌘N."), ui_style(16.0).color(THEME.sub), r.x + r.w * 0.5, y);
            return;
        };
        let shown = ui.ellipsize(&path, mono(11.0), bar.w - 120.0);
        ui.text(&shown, mono(11.0), bar.x + 22.0, bar.y + 18.0);
        let (status, color) = match self.save {
            SaveState::Clean => ("Salvo", THEME.mute),
            SaveState::Dirty | SaveState::Saving => ("Salvando", THEME.sub),
            SaveState::Conflict { .. } => ("Conflito", THEME.warn),
        };
        ui.text_right(status, ui_style(12.0).color(color), bar.right() - 22.0, bar.y + 16.0);
        if matches!(self.save, SaveState::Conflict { .. }) {
            let band = r.take_top(46.0);
            ui.s.fill(band, THEME.hover);
            ui.s.fill(Rect::new(band.x, band.bottom() - 1.0, band.w, 1.0), THEME.warn);
            ui.text("Esta nota mudou fora daqui enquanto você escrevia. Nada foi perdido.", ui_style(13.5), band.x + 22.0, band.y + 14.0);
            let b2 = Rect::new(band.right() - 150.0, band.y + 9.0, 134.0, 28.0);
            let b1 = Rect::new(b2.x - 150.0, band.y + 9.0, 140.0, 28.0);
            if ui.button(id("keep-mine"), b1, "Manter a minha") {
                self.resolve(0, tx);
            }
            if ui.button(id("use-disk"), b2, "Guardar as duas") {
                self.resolve(1, tx);
            }
        }
        let Some(ed) = self.editor.as_mut() else { return };
        let hover = ui.hover(r);
        if hover {
            ed.scroll -= ui.st.wheel.1;
        }
        if hover && ui.st.clicked {
            let (x, y) = ui.st.mouse;
            let at = ed.hit(x, y, ui.t);
            let shift = ui.st.mods.shift;
            if ui.st.mods.cmd {
                // ⌘ + clique segue o link: nota existente abre, pendente nasce.
                if let Some(target) = ed.link_at(at) {
                    if target.starts_with("http://") || target.starts_with("https://") {
                        let _ = std::process::Command::new("open").arg(&target).spawn();
                    } else {
                        let want = target.to_lowercase();
                        match model.notes.iter().find(|n| n.title.to_lowercase() == want || stem(&n.path).to_lowercase() == want || n.aliases.iter().any(|a| a.to_lowercase() == want)) {
                            Some(n) => {
                                let p = n.path.clone();
                                self.request_open(&p, tx);
                            }
                            None => {
                                let _ = tx.send(Cmd::CreateNote(target));
                            }
                        }
                        return;
                    }
                }
            } else if ui.st.clicks == 2 {
                ed.buf.select_word_at(at);
            } else {
                ed.buf.set_caret(at, shift);
            }
            self.drag = true;
        } else if self.drag && ui.st.down {
            let (x, y) = ui.st.mouse;
            let at = ed.hit(x, y, ui.t);
            ed.buf.set_caret(at, true);
        }
        if ui.st.released {
            self.drag = false;
        }
        ed.paint(r, ui.s, ui.t, true, ui.time);
    }

    fn margin(&mut self, ui: &mut Ui<'_>, r: Rect, model: &Model, tx: &Sender<Cmd>, out: &mut NotesOut) {
        let x = r.x + 20.0;
        let w = r.w - 40.0;
        let mut y = r.y + 56.0;
        let Some(path) = self.path.clone() else { return };
        let path = path.as_str();
        ui.section("Memórias desta nota", x, y);
        y += 24.0;
        let from: Vec<&Memory> = model.memories.iter().filter(|m| m.note.as_deref() == Some(path)).collect();
        if from.is_empty() {
            let h = ui.para("A IA ainda não aprendeu nada desta nota. Quando aprender, aparece aqui.", ui_style(13.0).color(THEME.mute), x, y, w);
            y += h + 12.0;
        }
        for m in from.iter().take(8) {
            let st = ui_style(13.5).color(THEME.sub);
            let t = ui.ellipsize(&m.title, st, w);
            let (fired, _) = ui.link(id(("note-memory", &m.id)), x, y, &t, st);
            if fired {
                out.open_memory = Some(m.id.clone());
            }
            y += 26.0;
        }
        if let Some(links) = model.backlinks.get(path) {
            y += 18.0;
            ui.section("Notas que apontam para esta", x, y);
            y += 24.0;
            for from in links.iter().take(10) {
                let title = model.notes.iter().find(|n| n.path == *from).map_or(from.as_str(), |n| n.title.as_str());
                let st = ui_style(13.5).color(THEME.sub);
                let t = ui.ellipsize(title, st, w);
                let (fired, _) = ui.link(id(("backlink", from)), x, y, &t, st);
                if fired {
                    self.request_open(from, tx);
                }
                y += 26.0;
            }
        }
    }
}

fn stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name)
}

impl Default for NotesView {
    fn default() -> Self {
        Self::new()
    }
}

fn edit_key(ed: &mut EditorView, k: Key, m: Mods, view_h: f32, clip: &mut crate::Clipboard) -> bool {
    let b = &mut ed.buf;
    let word = m.alt;
    let line = m.cmd;
    match k {
        Key::Left if line => b.motion(Move::LineStart, m.shift),
        Key::Right if line => b.motion(Move::LineEnd, m.shift),
        Key::Up if line => b.motion(Move::DocStart, m.shift),
        Key::Down if line => b.motion(Move::DocEnd, m.shift),
        Key::Left if word => b.motion(Move::WordLeft, m.shift),
        Key::Right if word => b.motion(Move::WordRight, m.shift),
        Key::Left => b.motion(Move::Left, m.shift),
        Key::Right => b.motion(Move::Right, m.shift),
        Key::Up => b.motion(Move::Up, m.shift),
        Key::Down => b.motion(Move::Down, m.shift),
        Key::Home => b.motion(Move::LineStart, m.shift),
        Key::End => b.motion(Move::LineEnd, m.shift),
        Key::PageUp => ed.scroll -= view_h * 0.9,
        Key::PageDown => ed.scroll += view_h * 0.9,
        Key::Backspace if word => b.delete_word_back(),
        Key::Backspace => b.backspace(),
        Key::Delete => b.delete_forward(),
        Key::Enter => b.newline(),
        Key::Tab => b.insert("  "),
        Key::Char('a') if m.cmd => b.select_all(),
        Key::Char('z') if m.cmd && m.shift => b.redo(),
        Key::Char('z') if m.cmd => b.undo(),
        Key::Char('c') if m.cmd => clip.set(&b.selected_text()),
        Key::Char('x') if m.cmd => {
            clip.set(&b.selected_text());
            b.insert("");
        }
        Key::Char('v') if m.cmd => {
            let t = clip.get();
            b.insert(&t);
        }
        Key::Char('b') if m.cmd => wrap(ed, "**"),
        Key::Char('i') if m.cmd => wrap(ed, "*"),
        _ => return false,
    }
    true
}

fn wrap(ed: &mut EditorView, mark: &str) {
    let t = ed.buf.selected_text();
    ed.buf.insert(&format!("{mark}{t}{mark}"));
}
