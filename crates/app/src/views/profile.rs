//! Perfil: o que a IA acha que sabe de você. Editar uma frase é a correção mais forte que existe.

use crate::command::Cmd;
use crate::model::Model;
use crate::theme::{mono, ui as ui_style, THEME};
use crate::ui::{id, Ui};
use ed::Buffer;
use gfx::{Align, Rect, Span};
use std::sync::mpsc::Sender;

#[derive(Default)]
pub struct ProfileView {
    pub editing: Option<(Option<String>, Buffer)>,
    content: f32,
}

impl ProfileView {
    pub fn commit(&mut self, tx: &Sender<Cmd>) {
        if let Some((memory, b)) = self.editing.take() {
            let text = b.text().trim().to_string();
            if !text.is_empty() {
                let _ = tx.send(Cmd::Correct { memory, text });
            }
        }
    }

    pub fn paint(&mut self, ui: &mut Ui<'_>, r: Rect, model: &Model, tx: &Sender<Cmd>) {
        ui.head(r.x + 20.0, r.y, "Perfil");
        let list = r.pad(0.0, 56.0, 0.0, 0.0);
        let col = 640f32.min(list.w - 48.0);
        let x = list.x + (list.w - col) * 0.5;
        let line_style = ui_style(16.0).leading(1.5);
        let off = ui.scroll(id("profile"), list, self.content);
        ui.s.push_clip(list);
        let top = list.y + 8.0 - off;
        let mut y = top;
        y += ui.para("O que a IA sabe sobre você. Clique numa frase para corrigir. A correção vale daqui para a frente, em toda sessão.", ui_style(15.0).color(THEME.sub), x, y, col);
        if model.profile.is_empty() {
            ui.para("A IA ainda sabe pouco sobre você. Trabalhe com ela, corrija quando errar, e este perfil se escreve sozinho.", ui_style(15.0).color(THEME.mute), x, y + 24.0, col);
        }
        let text_w = col - 92.0;
        for s in &model.profile {
            y += 32.0;
            ui.section(&s.title, x, y);
            y += 22.0;
            for l in &s.lines {
                ui.s.fill(Rect::new(x, y, col, 1.0), THEME.line);
                let editing_this = self.editing.as_ref().is_some_and(|(m, _)| m.as_deref() == Some(l.memory.as_str()));
                if editing_this {
                    let Some((_, b)) = self.editing.as_ref() else { continue };
                    let text = b.text();
                    let lay = ui.t.layout(&[Span::new(&text, line_style)], Some(col), Align::Left);
                    let h = lay.height.max(24.0);
                    ui.s.text(ui.t, &lay, x, y + 10.0);
                    let (cx, cy, ch) = lay.caret(b.line_byte(b.head()).1);
                    ui.s.fill(Rect::new(x + cx, y + 10.0 + cy + 3.0, 1.5, ch - 6.0), THEME.acc);
                    ui.s.fill(Rect::new(x, y + h + 13.0, col, 1.0), THEME.acc);
                    y += h + 20.0;
                    continue;
                }
                let h = ui.para_height(&l.text, line_style, text_w);
                let row = Rect::new(x, y, col, h + 20.0);
                let key = id(("profile", &l.memory, &l.text));
                let hover = ui.hover(row);
                let a = ui.anim(key ^ 5, hover as u32 as f32, 20.0);
                ui.para(&l.text, line_style.color(THEME.ink.mix(THEME.acc, a * 0.7)), x, y + 10.0, text_w);
                ui.text_right(l.authority.label(), mono(11.5), x + col, y + 13.0);
                if ui.pressed(key, row) {
                    self.commit(tx);
                    let mut b = Buffer::new(&l.text);
                    let end = b.len_chars();
                    b.set_caret(end, false);
                    self.editing = Some((Some(l.memory.clone()), b));
                }
                y += h + 20.0;
            }
        }
        ui.s.pop_clip();
        self.content = y - top + 64.0;
    }
}
