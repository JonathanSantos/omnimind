//! Ajustes: conexões com a IA, cofre e aparência.

use crate::command::Cmd;
use crate::model::{LinkState, Model};
use crate::theme::{is_dark, ui as ui_style, THEME};
use crate::ui::{id, Ui};
use gfx::Rect;
use std::sync::mpsc::Sender;

#[derive(Default)]
pub struct SettingsView {
    pub message: String,
    content: f32,
}

#[derive(Default)]
pub struct SettingsOut {
    pub toggle_theme: bool,
    pub open_vault: bool,
}

impl SettingsView {
    fn row(ui: &mut Ui<'_>, x: f32, y: f32, col: f32, title: &str, sub: &str, right: f32) -> f32 {
        ui.s.fill(Rect::new(x, y, col, 1.0), THEME.line);
        ui.text(title, ui_style(14.0).weight(500), x, y + 14.0);
        let h = ui.para(sub, ui_style(13.0).color(THEME.sub), x, y + 35.0, col - right);
        h + 50.0
    }

    pub fn paint(&mut self, ui: &mut Ui<'_>, r: Rect, model: &Model, tx: &Sender<Cmd>, clip: &mut crate::Clipboard) -> SettingsOut {
        let mut out = SettingsOut::default();
        ui.head(r.x + 20.0, r.y, "Ajustes");
        let list = r.pad(0.0, 56.0, 0.0, 0.0);
        let col = 640f32.min(list.w - 48.0);
        let x = list.x + (list.w - col) * 0.5;
        let off = ui.scroll(id("settings"), list, self.content);
        ui.s.push_clip(list);
        let top = list.y + 8.0 - off;
        let mut y = top;
        ui.section("Conexões", x, y);
        y += 24.0;
        for (name, slug, about) in [("Claude Code", "claude-code", "Lembra entre sessões, aprende quando você corrige."), ("GitHub Copilot", "copilot", "Lembra entre sessões neste projeto.")] {
            let on = model.links.iter().filter(|l| l.client == slug && l.state == LinkState::On).count();
            let seen = model.clients.iter().find(|c| c.name == slug).map_or(String::new(), |c| format!(" · última sessão {}", c.last_seen));
            let status = match on {
                0 => "Desligado".to_string(),
                1 => format!("Ligado em 1 projeto{seen}"),
                n => format!("Ligado em {n} projetos{seen}"),
            };
            let top_y = y;
            y += Self::row(ui, x, y, col, name, &format!("{about} {status}."), 330.0);
            let bin = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("omnimind").to_string_lossy().into_owned())).unwrap_or_else(|| "omnimind".into());
            let cmd = format!("\"{bin}\" connect {slug} --brain \"{}\" --project .", model.path);
            if ui.button(id(("copy-connect", slug)), Rect::new(x + col - 142.0, top_y + 14.0, 142.0, 34.0), "Copiar comando") {
                clip.set(&cmd);
                self.message = format!("Comando copiado. Rode na pasta do projeto: {cmd}");
            }
            if ui.button(id(("connect-here", slug)), Rect::new(x + col - 304.0, top_y + 14.0, 154.0, 34.0), "Conectar projeto…") {
                if let Some(p) = crate::pick_folder("Escolha a pasta do projeto") {
                    let _ = tx.send(Cmd::Connect { client: slug.to_string(), project: p });
                }
            }
        }
        if !self.message.is_empty() {
            y += 8.0 + ui.para(&self.message, ui_style(13.0).color(THEME.acc), x, y + 8.0, col) + 8.0;
        }
        y += 24.0;
        ui.section("Projetos", x, y);
        y += 24.0;
        if model.links.is_empty() {
            y += ui.para("Nenhum projeto ainda. Use Conectar projeto para ligar o primeiro.", ui_style(13.0).color(THEME.mute), x, y + 8.0, col) + 16.0;
        }
        for l in &model.links {
            let label = crate::stats::client_label(&l.client);
            let about = match l.state {
                LinkState::On => format!("{label} · Ligado"),
                LinkState::Off => format!("{label} · Desligado"),
                LinkState::Elsewhere => format!("{label} · Ligado a outro cofre"),
                LinkState::Missing => format!("{label} · Pasta não encontrada"),
            };
            let top_y = y;
            y += Self::row(ui, x, y, col, &l.name, &format!("{about}\n{}", l.project), 170.0);
            let key = (l.project.as_str(), l.client.as_str());
            let bar = Rect::new(x + col - 44.0, top_y + 16.0, 44.0, 24.0);
            let fixed = matches!(l.state, LinkState::Elsewhere | LinkState::Missing);
            if !fixed && ui.toggle(id(("link", key)), bar, l.state == LinkState::On) {
                let (client, project) = (l.client.clone(), l.project.clone());
                let _ = tx.send(if l.state == LinkState::On { Cmd::Disconnect { client, project } } else { Cmd::Connect { client, project } });
            }
            if l.state != LinkState::On {
                let at = if fixed { x + col - 96.0 } else { x + col - 148.0 };
                if ui.button(id(("forget", key)), Rect::new(at, top_y + 11.0, 96.0, 34.0), "Remover") {
                    let _ = tx.send(Cmd::ForgetLink { client: l.client.clone(), project: l.project.clone() });
                }
            }
        }
        y += 24.0;
        ui.section("Cofre", x, y);
        y += 24.0;
        let name = if model.name.is_empty() { "Cofre" } else { &model.name };
        let info = format!("{} · {} notas · {} memórias", model.path, model.notes.len(), model.memories.len());
        let top_y = y;
        y += Self::row(ui, x, y, col, name, &info, 110.0);
        if ui.button(id("open-vault"), Rect::new(x + col - 96.0, top_y + 14.0, 96.0, 34.0), "Trocar") {
            out.open_vault = true;
        }
        y += 8.0 + ui.para("Um cofre nunca vê outro. O servidor da IA nasce preso a uma pasta. Para o trabalho e a vida pessoal não se misturarem, use um cofre para cada.", ui_style(13.0).color(THEME.mute), x, y + 8.0, col);
        y += 32.0;
        ui.section("Aparência", x, y);
        y += 24.0;
        let top_y = y;
        y += Self::row(ui, x, y, col, "Tema", if is_dark() { "Escuro" } else { "Claro" }, 110.0);
        if ui.button(id("theme"), Rect::new(x + col - 96.0, top_y + 14.0, 96.0, 34.0), "Alternar") {
            out.toggle_theme = true;
        }
        ui.s.pop_clip();
        self.content = y - top + 64.0;
        out
    }
}
