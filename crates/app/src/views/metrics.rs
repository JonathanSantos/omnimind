//! Métricas: o que foi lido, o que falta e o que pede revisão, em 7, 30 e 90 dias.

use crate::model::Model;
use crate::stats::{Bar, Period, Ranked, Totals, PERIODS};
use crate::theme::{mono, ui as ui_style, THEME};
use crate::ui::{id, Ui};
use gfx::Rect;

#[derive(Default)]
pub struct MetricsView {
    period: usize,
    memories_tab: bool,
    content: f32,
}

#[derive(Default)]
pub struct MetricsOut {
    pub open_note: Option<String>,
    pub open_memory: Option<String>,
    pub review_held: bool,
    pub show_memories: bool,
    pub show_graph: bool,
    pub create_note: Option<String>,
    pub dream: bool,
}

fn pct(now: u32, before: u32) -> String {
    if before == 0 {
        return if now == 0 { "Sem mudança".into() } else { "Novo no período".into() };
    }
    let p = ((now as f32 - before as f32) / before as f32 * 100.0).round() as i32;
    format!("{}{p}% vs. período anterior", if p > 0 { "+" } else { "" })
}

fn pct_short(now: u32, before: u32) -> String {
    if before == 0 {
        return if now == 0 { "—".into() } else { "novo".into() };
    }
    let p = ((now as f32 - before as f32) / before as f32 * 100.0).round() as i32;
    format!("{}{p}%", if p > 0 { "+" } else { "" })
}

pub fn thousands(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// Topo arredondado do eixo, sempre com quatro passos.
fn nice_top(v: u32) -> (u32, u32) {
    let v = v.max(4) as f32;
    let pow = 10f32.powf(v.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|k| k * pow).find(|s| s * 4.0 >= v).unwrap_or(pow * 10.0);
    let step = step.max(1.0) as u32;
    (step, step * 4)
}

impl MetricsView {
    fn stat(ui: &mut Ui<'_>, r: Rect, label: &str, value: &str, delta: &str) {
        ui.s.fill(Rect::new(r.x, r.y, r.w, 1.0), THEME.line);
        ui.text(label, ui_style(13.0).color(THEME.sub), r.x, r.y + 12.0);
        ui.text(value, ui_style(30.0).weight(600), r.x, r.y + 30.0);
        let st = ui_style(12.5).color(THEME.mute);
        let d = ui.ellipsize(delta, st, r.w);
        ui.text(&d, st, r.x, r.y + 74.0);
    }

    fn stats(ui: &mut Ui<'_>, x: f32, y: f32, col: f32, p: &Period) -> f32 {
        let (n, b): (&Totals, &Totals) = (&p.now, &p.before);
        let columns = if col >= 860.0 { 4 } else { 2 };
        let gap = 28.0;
        let w = (col - gap * (columns - 1) as f32) / columns as f32;
        let rate = n.hit_rate().map_or("—".to_string(), |r| format!("{}%", (r * 100.0).round() as i32));
        let rate_delta = match (n.hit_rate(), b.hit_rate()) {
            (Some(a), Some(c)) => {
                let d = ((a - c) * 100.0).round() as i32;
                format!("{}{d} pontos vs. período anterior", if d >= 0 { "+" } else { "" })
            }
            (None, _) => "Nenhuma busca da IA".to_string(),
            _ => "Sem período anterior".to_string(),
        };
        let cells = [
            ("Leituras", thousands(n.reads), pct(n.reads, b.reads)),
            ("Memórias novas", thousands(n.memories), pct(n.memories, b.memories)),
            ("Correções suas", thousands(n.fixes), pct(n.fixes, b.fixes)),
            ("Buscas com resposta", rate, rate_delta),
        ];
        for (i, c) in cells.iter().enumerate() {
            let r = Rect::new(x + (i % columns) as f32 * (w + gap), y + (i / columns) as f32 * 112.0, w, 100.0);
            Self::stat(ui, r, c.0, &c.1, &c.2);
        }
        cells.len().div_ceil(columns) as f32 * 112.0
    }

    fn chart(ui: &mut Ui<'_>, r: Rect, p: &Period) -> f32 {
        ui.text(if p.weekly { "Leituras por semana" } else { "Leituras por dia" }, ui_style(15.0).weight(600), r.x, r.y + 4.0);
        let right = r.right();
        ui.s.round(Rect::new(right - 96.0, r.y + 9.0, 9.0, 9.0), 3.0, THEME.acc);
        ui.text("IA", ui_style(12.5).color(THEME.sub), right - 82.0, r.y + 5.0);
        ui.s.round(Rect::new(right - 52.0, r.y + 9.0, 9.0, 9.0), 3.0, THEME.mute);
        ui.text("Você", ui_style(12.5).color(THEME.sub), right - 38.0, r.y + 5.0);
        let plot = Rect::new(r.x + 46.0, r.y + 44.0, r.w - 46.0, 196.0);
        let max = p.bars.iter().map(|b: &Bar| b.ai + b.you).max().unwrap_or(0);
        let (step, top) = nice_top(max);
        for k in 0..=4 {
            let y = plot.bottom() - plot.h * k as f32 / 4.0;
            ui.s.fill(Rect::new(r.x, y, r.w, 1.0), THEME.line);
            ui.s.fill(Rect::new(r.x, y - 8.0, 42.0, 16.0), THEME.bg);
            ui.text(&thousands(step * k), mono(11.0), r.x, y - 8.0);
        }
        let n = p.bars.len().max(1) as f32;
        let cw = plot.w / n;
        let gap = (cw * 0.24).clamp(2.0, 6.0);
        // Marca fina: poucas barras não viram blocos.
        let bar = (cw - gap).min(56.0);
        let mut hovered = None;
        for (i, b) in p.bars.iter().enumerate() {
            let cell = Rect::new(plot.x + i as f32 * cw, plot.y, cw, plot.h);
            if ui.hover(cell) {
                ui.s.fill(cell, THEME.hover);
                hovered = Some(i);
            }
            let (ax, aw) = (cell.x + (cw - bar) * 0.5, bar);
            let ka = id(("bar-ai", p.days, i));
            let kh = id(("bar-you", p.days, i));
            let ha = ui.anim_from(ka, 0.0, plot.h * b.ai as f32 / top as f32, 16.0);
            let hy = ui.anim_from(kh, 0.0, plot.h * b.you as f32 / top as f32, 16.0);
            if ha > 0.3 {
                ui.s.round(Rect::new(ax, plot.bottom() - ha, aw, ha), 0.0, THEME.acc);
            }
            if hy > 0.3 {
                ui.s.round(Rect::new(ax, plot.bottom() - ha - hy - 2.0, aw, hy), (aw * 0.25).min(4.0).min(hy * 0.5), THEME.mute);
            }
        }
        if let (Some(a), Some(z)) = (p.bars.first(), p.bars.last()) {
            let st = mono(11.0);
            let trim = |s: &str| s.strip_prefix("Semana de ").unwrap_or(s).to_string();
            ui.text(&trim(&a.label), st, plot.x, plot.bottom() + 8.0);
            ui.text_center(&trim(&p.bars[p.bars.len() / 2].label), st, plot.x + plot.w * 0.5, plot.bottom() + 8.0);
            ui.text_right(&trim(&z.label), st, plot.right(), plot.bottom() + 8.0);
        }
        if let Some(i) = hovered {
            let b = &p.bars[i];
            let (l1, l2) = (format!("IA: {}", thousands(b.ai)), format!("Você: {}", thousands(b.you)));
            let w = ui.line(&b.label, ui_style(12.5).weight(600)).width.max(ui.line(&l1, ui_style(12.5)).width).max(ui.line(&l2, ui_style(12.5)).width) + 20.0;
            let cx = plot.x + (i as f32 + 0.5) * cw;
            let x = if cx + w + 10.0 > plot.right() { cx - w - 8.0 } else { cx + 8.0 };
            let card = Rect::new(x, plot.y + 4.0, w, 66.0);
            ui.s.shadow(card, 8.0, 16.0, THEME.shadow);
            ui.s.panel(card, 8.0, THEME.panel, THEME.line, 1.0);
            ui.text(&b.label, ui_style(12.5).weight(600), card.x + 10.0, card.y + 7.0);
            ui.text(&l1, ui_style(12.5).color(THEME.sub), card.x + 10.0, card.y + 26.0);
            ui.text(&l2, ui_style(12.5).color(THEME.sub), card.x + 10.0, card.y + 44.0);
        }
        272.0
    }

    fn ranked(&mut self, ui: &mut Ui<'_>, x: f32, y: f32, w: f32, p: &Period, out: &mut MetricsOut) -> f32 {
        ui.text("Mais lidas", ui_style(15.0).weight(600), x, y + 4.0);
        let tabs = [("Notas", false), ("Memórias", true)];
        let widths: f32 = tabs.iter().map(|t| ui.line(t.0, ui_style(13.0).weight(600)).width + 20.0).sum();
        let mut cx = x + w - widths - 4.0;
        for (label, mem) in tabs {
            let (fired, cw) = ui.chip(id(("top-tab", label)), cx, y - 2.0, label, self.memories_tab == mem);
            if fired {
                self.memories_tab = mem;
            }
            cx += cw + 4.0;
        }
        let rows: &[Ranked] = if self.memories_tab { &p.top_memories } else { &p.top_notes };
        let mut ry = y + 38.0;
        if rows.is_empty() {
            ui.text("Nada lido neste período.", ui_style(14.0).color(THEME.mute), x, ry + 8.0);
            return 104.0;
        }
        let peak = rows[0].count.max(1) as f32;
        for (i, r) in rows.iter().enumerate() {
            let row = Rect::new(x, ry, w, 48.0);
            let key = id(("rank", &r.id));
            if ui.pressed(key, row) {
                if r.is_note {
                    out.open_note = Some(r.id.clone());
                } else {
                    out.open_memory = Some(r.id.clone());
                }
            }
            if ui.hover(row) {
                ui.s.fill(row, THEME.hover);
            }
            ui.s.fill(Rect::new(x, ry, w, 1.0), THEME.line);
            ui.text_right(&(i + 1).to_string(), mono(12.0), x + 16.0, ry + 8.0);
            let st = ui_style(14.0).weight(500);
            let t = ui.ellipsize(&r.title, st, w - 96.0);
            ui.text(&t, st, x + 28.0, ry + 7.0);
            ui.text_right(&thousands(r.count), ui_style(14.0).weight(500), x + w, ry + 13.0);
            let room = w - 96.0;
            let full = room * r.count as f32 / peak;
            let ia = full * r.by_ai as f32 / r.count.max(1) as f32;
            let k = id(("rank-bar", &r.id, p.days));
            let a = ui.anim_from(k, 0.0, 1.0, 14.0);
            if ia > 0.5 {
                ui.s.fill(Rect::new(x + 28.0, ry + 33.0, ia * a, 6.0), THEME.acc);
            }
            if full - ia > 0.5 {
                ui.s.round(Rect::new(x + 28.0 + (ia + 2.0) * a, ry + 33.0, (full - ia) * a, 6.0), 2.0, THEME.mute);
            }
            ry += 48.0;
        }
        ry - y
    }

    fn themes(ui: &mut Ui<'_>, x: f32, y: f32, w: f32, p: &Period, out: &mut MetricsOut) -> f32 {
        ui.text("Temas mais lidos", ui_style(15.0).weight(600), x, y + 4.0);
        let mut ry = y + 38.0;
        if p.themes.is_empty() {
            ui.text("Os temas aparecem quando o cofre tiver conteúdo.", ui_style(14.0).color(THEME.mute), x, ry + 8.0);
            return 104.0;
        }
        let peak = p.themes[0].count.max(1) as f32;
        for t in &p.themes {
            let row = Rect::new(x, ry, w, 40.0);
            if ui.pressed(id(("theme-row", &t.name)), row) {
                out.show_graph = true;
            }
            if ui.hover(row) {
                ui.s.fill(row, THEME.hover);
            }
            ui.s.fill(Rect::new(x, ry, w, 1.0), THEME.line);
            let st = ui_style(14.0).weight(500);
            let name = ui.ellipsize(&t.name, st, 128.0);
            ui.text(&name, st, x, ry + 10.0);
            let track = w - 250.0;
            let k = id(("theme-bar", &t.name, p.days));
            let a = ui.anim_from(k, 0.0, 1.0, 14.0);
            ui.s.round(Rect::new(x + 136.0, ry + 17.0, (track * t.count as f32 / peak * a).max(1.0), 6.0), 2.0, THEME.themes[t.slot as usize]);
            ui.text_right(&thousands(t.count), ui_style(14.0), x + w - 58.0, ry + 10.0);
            ui.text_right(&pct_short(t.count, t.before), mono(12.0), x + w, ry + 12.0);
            ry += 40.0;
        }
        ry - y
    }

    #[allow(clippy::too_many_arguments)]
    fn todo(ui: &mut Ui<'_>, x: f32, y: f32, w: f32, value: &str, title: &str, text: &str, action: &str, key: u64) -> bool {
        ui.s.fill(Rect::new(x, y, w, 1.0), THEME.line);
        ui.text(value, ui_style(20.0).weight(600), x, y + 13.0);
        let room = w - 170.0;
        let st = ui_style(14.0).weight(500);
        let t = ui.ellipsize(title, st, room);
        ui.text(&t, st, x + 56.0, y + 9.0);
        let st2 = ui_style(13.0).color(THEME.mute);
        let s = ui.ellipsize(text, st2, room);
        ui.text(&s, st2, x + 56.0, y + 29.0);
        let bw = ui.line(action, ui_style(13.5).weight(500)).width + 26.0;
        ui.button(key, Rect::new(x + w - bw, y + 11.0, bw, 34.0), action)
    }

    fn review(ui: &mut Ui<'_>, x: f32, y: f32, w: f32, m: &Model, p: &Period, out: &mut MetricsOut) -> f32 {
        ui.text("Para revisar", ui_style(15.0).weight(600), x, y + 4.0);
        let mut ry = y + 38.0;
        let stats = &m.metrics;
        if m.quarantined > 0 && Self::todo(ui, x, ry, w, &m.quarantined.to_string(), "Em revisão", "Veio de fora ou parece uma ordem. A IA só usa depois de aprovada.", "Revisar", id("todo-held")) {
            out.review_held = true;
        }
        if m.quarantined > 0 {
            ry += 56.0;
        }
        if stats.stale > 0 {
            if Self::todo(ui, x, ry, w, &thousands(stats.stale), "Desatualizadas", "O tempo ou algo mais novo tirou a validade.", "Ver", id("todo-stale")) {
                out.show_memories = true;
            }
            ry += 56.0;
        }
        if p.unread > 0 {
            let title = format!("Sem leitura em {} dias", p.days);
            if Self::todo(ui, x, ry, w, &thousands(p.unread), &title, "Candidatas a arquivar.", "Ver", id("todo-unread")) {
                out.show_memories = true;
            }
            ry += 56.0;
        }
        if stats.orphans > 0 {
            if Self::todo(ui, x, ry, w, &thousands(stats.orphans), "Notas sem ligação", "Nada se conecta a elas.", "Abrir grafo", id("todo-orphans")) {
                out.show_graph = true;
            }
            ry += 56.0;
        }
        if !m.agenda.is_empty() {
            let n = m.agenda.len().to_string();
            let first = m.agenda[0].text.clone();
            if Self::todo(ui, x, ry, w, &n, "Para melhorar com a IA", &first, "Copiar /sonhar", id("todo-dream")) {
                out.dream = true;
            }
            ry += 56.0;
        }
        if ry == y + 38.0 {
            ui.text("Nada pendente neste período.", ui_style(14.0).color(THEME.mute), x, ry + 8.0);
            return 104.0;
        }
        ry - y
    }

    fn gaps(ui: &mut Ui<'_>, x: f32, y: f32, w: f32, p: &Period, out: &mut MetricsOut) -> f32 {
        ui.text("Buscas sem resposta", ui_style(15.0).weight(600), x, y + 4.0);
        let mut ry = y + 38.0;
        if p.gaps.is_empty() {
            ui.text("A IA encontrou tudo o que procurou.", ui_style(14.0).color(THEME.mute), x, ry + 8.0);
            return 104.0;
        }
        for g in &p.gaps {
            let times = if g.count == 1 { "Procurada 1 vez".to_string() } else { format!("Procurada {} vezes", thousands(g.count)) };
            if Self::todo(ui, x, ry, w, &thousands(g.count), &g.query, &times, "Criar nota", id(("gap", &g.query))) {
                out.create_note = Some(g.query.clone());
            }
            ry += 56.0;
        }
        ry - y
    }

    fn tools(ui: &mut Ui<'_>, x: f32, y: f32, w: f32, p: &Period) -> f32 {
        ui.text("Quem usa", ui_style(15.0).weight(600), x, y + 4.0);
        let cols = [("Sessões", w - 150.0), ("Leituras", w - 70.0), ("Escritas", w)];
        let mut ry = y + 38.0;
        if p.tools.is_empty() {
            ui.text("Nenhuma IA usou este cofre no período.", ui_style(14.0).color(THEME.mute), x, ry + 8.0);
            return 104.0;
        }
        ui.text("Origem", ui_style(12.5).color(THEME.mute), x, ry);
        for (c, right) in cols {
            ui.text_right(c, ui_style(12.5).color(THEME.mute), x + right, ry);
        }
        ry += 24.0;
        for t in &p.tools {
            ui.s.fill(Rect::new(x, ry, w, 1.0), THEME.line);
            ui.text(&t.name, ui_style(14.0).weight(500), x, ry + 11.0);
            for (v, (_, right)) in [t.sessions, t.reads, t.writes].iter().zip(cols) {
                ui.text_right(&thousands(*v), ui_style(14.0), x + right, ry + 11.0);
            }
            ry += 42.0;
        }
        ry - y
    }

    fn rhythm(ui: &mut Ui<'_>, x: f32, y: f32, w: f32, m: &Model) -> f32 {
        let s = &m.metrics;
        ui.text("Ritmo de escrita", ui_style(15.0).weight(600), x, y + 4.0);
        ui.text_right("Últimas 13 semanas", ui_style(12.5).color(THEME.sub), x + w, y + 5.0);
        let top = y + 38.0;
        const CELL: f32 = 14.0;
        const GAP: f32 = 3.0;
        for (i, &v) in s.heat.iter().enumerate() {
            let at = i + s.heat_pad;
            let (cx, cy) = (x + (at / 7) as f32 * (CELL + GAP), top + (at % 7) as f32 * (CELL + GAP));
            let c = match v.min(4) {
                0 => THEME.hover,
                1 => THEME.hover.mix(THEME.acc, 0.3),
                2 => THEME.hover.mix(THEME.acc, 0.55),
                3 => THEME.hover.mix(THEME.acc, 0.78),
                _ => THEME.acc,
            };
            ui.s.round(Rect::new(cx, cy, CELL, CELL), 3.0, c);
        }
        let fx = x + 14.0 * (CELL + GAP) + 24.0;
        let facts = [(s.streak.to_string(), "dias seguidos"), (s.active.to_string(), "dias ativos"), (if s.best_day.is_empty() { "—".to_string() } else { s.best_day.to_string() }, "dia mais forte")];
        if fx + 100.0 < x + w {
            for (i, (v, label)) in facts.iter().enumerate() {
                let ry = top + i as f32 * 42.0;
                ui.text(v, ui_style(19.0).weight(600), fx, ry - 4.0);
                ui.text(label, ui_style(12.5).color(THEME.mute), fx, ry + 20.0);
            }
        }
        38.0 + 7.0 * (CELL + GAP) + 8.0
    }

    pub fn paint(&mut self, ui: &mut Ui<'_>, r: Rect, model: &Model) -> MetricsOut {
        let mut out = MetricsOut::default();
        let mut x = r.x + 20.0 + ui.head(r.x + 20.0, r.y, "Métricas") + 16.0;
        for (i, d) in PERIODS.iter().enumerate() {
            let (fired, w) = ui.chip(id(("period", d)), x, r.y + 14.0, &format!("{d} dias"), self.period == i);
            if fired {
                self.period = i;
            }
            x += w + 4.0;
        }
        let list = r.pad(0.0, 56.0, 0.0, 0.0);
        let Some(p) = model.metrics.periods.get(self.period).cloned() else {
            ui.text_center("As métricas aparecem depois da primeira sessão.", ui_style(15.0).color(THEME.sub), list.x + list.w * 0.5, list.y + list.h * 0.36);
            return out;
        };
        let col = 1120f32.min(list.w - 48.0);
        let x = list.x + (list.w - col) * 0.5;
        let off = ui.scroll(id("metrics"), list, self.content);
        ui.s.push_clip(list);
        let top = list.y + 8.0 - off;
        let mut y = top;
        y += Self::stats(ui, x, y, col, &p) + 24.0;
        y += Self::chart(ui, Rect::new(x, y, col, 272.0), &p) + 24.0;
        let two = col >= 760.0;
        let gap = 48.0;
        let half = if two { (col - gap) * 0.5 } else { col };
        let (x2, step) = if two { (x + half + gap, false) } else { (x, true) };
        let a = self.ranked(ui, x, y, half, &p, &mut out);
        let b = if step { Self::themes(ui, x2, y + a + 32.0, half, &p, &mut out) + a + 32.0 } else { Self::themes(ui, x2, y, half, &p, &mut out) };
        y += a.max(b) + 40.0;
        let a = Self::review(ui, x, y, half, model, &p, &mut out);
        let b = if step { Self::gaps(ui, x2, y + a + 32.0, half, &p, &mut out) + a + 32.0 } else { Self::gaps(ui, x2, y, half, &p, &mut out) };
        y += a.max(b) + 40.0;
        let a = Self::tools(ui, x, y, half, &p);
        let b = if step { Self::rhythm(ui, x2, y + a + 32.0, half, model) + a + 32.0 } else { Self::rhythm(ui, x2, y, half, model) };
        y += a.max(b);
        ui.s.pop_clip();
        self.content = y - top + 72.0;
        out
    }
}
