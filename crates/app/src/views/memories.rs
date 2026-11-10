//! Memórias: tudo o que a IA guarda, filtrável. A revisão de retidas mora aqui.

use crate::model::{MemKind, MemState, Memory, Model};
use crate::theme::{mono, ui as ui_style, THEME};
use crate::ui::{id, Ui};
use gfx::Rect;

const ROW: f32 = 46.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Kind(MemKind),
    Review,
}

#[derive(Default)]
pub struct MemoriesView {
    pub filter: Filter,
}

fn shown(m: &Memory) -> bool {
    !matches!(m.state, MemState::Archived | MemState::Superseded)
}

impl MemoriesView {
    /// Devolve a memória clicada.
    pub fn paint(&mut self, ui: &mut Ui<'_>, area: Rect, model: &Model, selected: Option<&str>) -> Option<String> {
        let mut x = area.x + 20.0 + ui.head(area.x + 20.0, area.y, "Memórias") + 16.0;
        let names: [(Filter, &str); 6] = [
            (Filter::All, "Todas"),
            (Filter::Kind(MemKind::Decision), "Decisões"),
            (Filter::Kind(MemKind::Preference), "Preferências"),
            (Filter::Kind(MemKind::Procedure), "Procedimentos"),
            (Filter::Kind(MemKind::Concept), "Conceitos"),
            (Filter::Kind(MemKind::Episode), "Episódios"),
        ];
        for (f, label) in names {
            let (fired, w) = ui.chip(id(("mem-filter", label)), x, area.y + 14.0, label, self.filter == f);
            if fired {
                self.filter = f;
            }
            x += w + 4.0;
        }
        if model.quarantined > 0 {
            let t = if model.quarantined == 1 { "1 em revisão".to_string() } else { format!("{} em revisão", model.quarantined) };
            let st = ui_style(13.0).weight(500).color(THEME.acc);
            let l = ui.line(&t, st);
            let right = area.right() - 20.0;
            let r = Rect::new(right - l.width - 18.0, area.y + 14.0, l.width + 18.0, 28.0);
            if ui.pressed(id("mem-review"), r) {
                self.filter = if self.filter == Filter::Review { Filter::All } else { Filter::Review };
            }
            ui.s.round(Rect::new(r.x, r.y + 10.0, 7.0, 7.0), 3.5, THEME.acc);
            ui.draw(&l, r.x + 14.0, r.y + 6.0);
        }
        let rows: Vec<&Memory> = model
            .memories
            .iter()
            .filter(|m| match self.filter {
                Filter::All => shown(m),
                Filter::Kind(k) => shown(m) && m.kind == k,
                Filter::Review => m.state == MemState::Quarantined,
            })
            .collect();
        let list = area.pad(0.0, 56.0, 0.0, 0.0);
        if rows.is_empty() {
            let t = if model.memories.is_empty() { "A IA ainda não guardou nada. Conecte sua IA em Ajustes." } else { "Nada neste filtro." };
            ui.text_center(t, ui_style(15.0).color(THEME.sub), list.x + list.w * 0.5, list.y + list.h * 0.36);
            return None;
        }
        let off = ui.scroll(id("mem-list"), list, rows.len() as f32 * ROW);
        ui.s.push_clip(list);
        let narrow = list.w < 680.0;
        let (first, last) = ((off / ROW) as usize, ((off + list.h) / ROW) as usize + 1);
        let mut picked = None;
        for (i, m) in rows.iter().enumerate().take(last.min(rows.len())).skip(first) {
            let y = list.y + i as f32 * ROW - off;
            let r = Rect::new(list.x, y, list.w, ROW);
            let key = id(("mem-row", &m.id));
            if ui.pressed(key, r) {
                picked = Some(m.id.clone());
            }
            let on = selected == Some(m.id.as_str());
            let hover = ui.hover(r);
            let a = ui.anim(key ^ 3, (hover || on) as u32 as f32, 22.0);
            if a > 0.01 {
                ui.s.fill(r, (if on { THEME.sel } else { THEME.hover }).alpha(a));
            }
            ui.s.fill(Rect::new(r.x, y, r.w, 1.0), THEME.line);
            ui.text(m.kind.label(), ui_style(13.0).color(THEME.sub), r.x + 20.0, y + 13.0);
            let st = ui_style(14.0).weight(500);
            let room = if narrow { r.w - 170.0 } else { r.w - 370.0 };
            let t = ui.ellipsize(&m.title, st, room);
            ui.text(&t, st, r.x + 130.0, y + 12.0);
            if !narrow {
                let source = if m.state == MemState::Active { m.authority.label() } else { m.state.label() };
                let color = if m.state == MemState::Quarantined { THEME.acc } else { THEME.sub };
                ui.text(source, ui_style(13.0).color(color), r.right() - 216.0, y + 13.0);
                ui.text_right(&m.created, mono(12.0), r.right() - 20.0, y + 14.0);
            }
        }
        ui.s.pop_clip();
        picked
    }
}
