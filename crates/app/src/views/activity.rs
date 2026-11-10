//! Atividade: cada sessão com tudo o que a IA mudou e o motivo. Qualquer mudança pode sair.

use crate::command::Cmd;
use crate::model::{Change, Model, Session};
use crate::theme::{ui as ui_style, THEME};
use crate::ui::{id, Ui};
use gfx::Rect;
use std::collections::HashSet;
use std::sync::mpsc::Sender;

const COL: f32 = 720.0;

#[derive(Default)]
pub struct ActivityView {
    open: HashSet<String>,
    content: f32,
}

impl ActivityView {
    pub fn paint(&mut self, ui: &mut Ui<'_>, r: Rect, model: &Model, tx: &Sender<Cmd>) -> bool {
        ui.head(r.x + 20.0, r.y, "Atividade");
        let list = r.pad(0.0, 56.0, 0.0, 0.0);
        let col = COL.min(list.w - 48.0);
        let x = list.x + (list.w - col) * 0.5;
        if model.sessions.is_empty() {
            let msg = "Nenhuma sessão ainda. Depois da primeira conversa com a IA ligada a este cofre, ela aparece aqui.";
            let w = col.min(460.0);
            let px = list.x + (list.w - w) * 0.5;
            let y = list.y + list.h * 0.3;
            let h = ui.para(msg, ui_style(15.0).color(THEME.sub), px, y, w);
            return ui.button(id("activity-connect"), Rect::new(px, y + h + 16.0, 160.0, 34.0), "Conectar sua IA");
        }
        let off = ui.scroll(id("activity"), list, self.content);
        ui.s.push_clip(list);
        let top = list.y + 8.0 - off;
        let mut y = top;
        let mut day = "";
        for s in &model.sessions {
            if s.day != day {
                day = &s.day;
                y += 16.0;
                ui.section(day, x, y);
                y += 24.0;
            }
            y = self.session(ui, x, y, col, s, tx, list);
        }
        ui.s.pop_clip();
        self.content = y - top + 64.0;
        false
    }

    fn diff_height(&self, ui: &mut Ui<'_>, c: &Change, w: f32) -> f32 {
        let st = ui_style(14.0).leading(1.55);
        let mut h = 10.0 + ui.para_height(&c.after, st, w);
        if !c.before.is_empty() {
            h += ui.para_height(&c.before, st, w) + 6.0;
        }
        if !c.reason.is_empty() {
            h += ui.para_height(&c.reason, ui_style(12.5), w) + 8.0;
        }
        h
    }

    #[allow(clippy::too_many_arguments)]
    fn session(&mut self, ui: &mut Ui<'_>, x: f32, mut y: f32, col: f32, s: &Session, tx: &Sender<Cmd>, view: Rect) -> f32 {
        let dw = col - 94.0;
        let mut h = 48.0 + s.changes.len() as f32 * 34.0 + 12.0;
        for c in &s.changes {
            if self.open.contains(&c.event) {
                h += self.diff_height(ui, c, dw);
            }
        }
        if y > view.bottom() || y + h < view.y {
            return y + h;
        }
        let end = y + h;
        ui.s.fill(Rect::new(x, y, col, 1.0), THEME.line);
        let w = ui.text(&s.client, ui_style(14.0).weight(600), x, y + 14.0);
        let reads = if s.reads == 1 { "1 leitura".to_string() } else { format!("{} leituras", s.reads) };
        let meta = if s.project.is_empty() { format!("{} · {reads}", s.time) } else { format!("{} · {} · {reads}", s.time, s.project) };
        ui.text(&meta, ui_style(13.0).color(THEME.sub), x + w + 10.0, y + 15.0);
        let live: Vec<_> = s.changes.iter().filter(|c| c.undoable && !c.undone).collect();
        if !live.is_empty() {
            let st = ui_style(13.0).color(THEME.mute);
            let w = ui.line("Desfazer sessão", st).width;
            let (fired, _) = ui.link(id(("undo-session", &s.session)), x + col - w, y + 15.0, "Desfazer sessão", st);
            if fired {
                let _ = tx.send(Cmd::UndoSession(s.session.clone()));
            }
        }
        y += 46.0;
        for c in &s.changes {
            let row = Rect::new(x, y, col, 34.0);
            let key = id(("change", &c.event));
            let hover = ui.hover(row);
            if ui.pressed(key, Rect::new(x, y, col - 90.0, 34.0)) && !self.open.remove(&c.event) {
                self.open.insert(c.event.clone());
            }
            let dim = if c.undone { 0.42 } else { 1.0 };
            ui.text(&c.verb, ui_style(13.0).color(THEME.mute.alpha(dim)), x, y + 8.0);
            let st = ui_style(14.0).color(THEME.ink.alpha(dim));
            let t = ui.ellipsize(&c.title, st, col - 190.0);
            let tw = ui.text(&t, st, x + 94.0, y + 7.0);
            if c.undone {
                ui.s.fill(Rect::new(x + 94.0, y + 17.0, tw, 1.0), THEME.mute);
                ui.text_right("Desfeito", ui_style(13.0).color(THEME.mute), x + col, y + 8.0);
            } else if c.undoable && hover {
                let st = ui_style(13.0).color(THEME.sub);
                let w = ui.line("Desfazer", st).width;
                let (fired, _) = ui.link(key ^ 9, x + col - w, y + 8.0, "Desfazer", st);
                if fired {
                    let _ = tx.send(Cmd::Undo(c.event.clone()));
                }
            }
            y += 34.0;
            if self.open.contains(&c.event) {
                let (dx, w) = (x + 94.0, dw);
                let st = ui_style(14.0).leading(1.55);
                if !c.reason.is_empty() {
                    y += ui.para(&c.reason, ui_style(12.5).color(THEME.mute), dx, y, w) + 8.0;
                }
                if !c.before.is_empty() {
                    y += ui.para(&c.before, st.color(THEME.mute), dx, y, w) + 6.0;
                }
                y += ui.para(&c.after, st, dx, y, w) + 10.0;
            }
        }
        end
    }
}
