//! Thread de dados: único lugar do app que toca o núcleo. O quadro nunca espera disco.

use crate::command::{Cmd, Hit, Msg};
use crate::model::*;
use crate::demo;
use crate::stats;
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use omnimind_core as core;
use omnimind_core::{journal, Actor, Audience, Brain, Change as CoreChange, Event, EventKind, MemoryState, MemoryType, SaveResult, SearchQuery};

const POLL: Duration = Duration::from_millis(250);

fn config_dir() -> PathBuf {
    crate::platform::config_dir()
}

fn config_file() -> PathBuf {
    config_dir().join("last-brain")
}

/// Cofre lembrado. Quem vem de antes da troca de nome ainda tem o registro na pasta antiga.
fn remembered_brain() -> Option<String> {
    let old = config_dir().with_file_name("Varzea").join("last-brain");
    let saved = std::fs::read_to_string(config_file()).or_else(|_| std::fs::read_to_string(old)).ok().map(|s| s.trim().to_string());
    let old_vault = crate::platform::home_dir().join("Varzea");
    saved.or_else(|| old_vault.join("brain.toml").is_file().then(|| old_vault.to_string_lossy().into_owned()))
}

/// Tema escolhido em Ajustes. Sem escolha, segue o sistema.
pub fn load_theme() -> Option<bool> {
    match std::fs::read_to_string(config_dir().join("theme")).ok()?.trim() {
        "dark" => Some(true),
        "light" => Some(false),
        _ => None,
    }
}

pub fn save_theme(dark: bool) {
    let d = config_dir();
    let _ = std::fs::create_dir_all(&d);
    let _ = std::fs::write(d.join("theme"), if dark { "dark" } else { "light" });
}

fn remember_brain(p: &Path) {
    let f = config_file();
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(f, p.to_string_lossy().as_bytes());
}

fn mem_kind(t: MemoryType) -> MemKind {
    match t {
        MemoryType::Decision => MemKind::Decision,
        MemoryType::Procedure => MemKind::Procedure,
        MemoryType::Concept => MemKind::Concept,
        MemoryType::Episode => MemKind::Episode,
        MemoryType::Preference => MemKind::Preference,
    }
}

fn authority(a: core::Authority) -> Authority {
    match a {
        core::Authority::Person => Authority::Person,
        core::Authority::Agent => Authority::Agent,
        core::Authority::External => Authority::External,
    }
}

fn state(s: MemoryState) -> MemState {
    match s {
        MemoryState::Active => MemState::Active,
        MemoryState::Stale => MemState::Stale,
        MemoryState::Expired => MemState::Expired,
        MemoryState::Superseded => MemState::Superseded,
        MemoryState::Quarantined => MemState::Quarantined,
        MemoryState::Archived => MemState::Archived,
    }
}

fn origin_label(k: core::OriginKind) -> &'static str {
    match k.as_str() {
        "correction" => "correção",
        "request" => "pedido",
        "session" => "sessão",
        "note" => "nota",
        "tool" => "ferramenta",
        "consolidation" => "consolidação",
        "import" => "importação",
        _ => "outra",
    }
}

/// Dia em texto de tela: `Hoje`, `Ontem` ou `31 out`. O diário guarda UTC; a tela mostra a hora local.
fn day_text(ts: &str, clock: Clock) -> String {
    let Some((a, _)) = stats::local_time(ts, clock.offset) else { return ts.to_string() };
    match clock.today - a {
        0 => "Hoje".into(),
        1 => "Ontem".into(),
        _ => stats::day_label(a),
    }
}

fn hour_text(ts: &str, clock: Clock) -> String {
    stats::local_time(ts, clock.offset).map_or(String::new(), |(_, m)| format!("{:02}:{:02}", m / 60, m % 60))
}

#[derive(Clone, Copy)]
struct Clock {
    today: i64,
    offset: i64,
}

/// Tira o frontmatter de um texto de memória guardado no diário.
fn body_of(t: &str) -> String {
    if let Some(rest) = t.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            return rest[end + 5..].trim().to_string();
        }
    }
    t.trim().to_string()
}

const MONTH_NAMES: [&str; 12] = ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"];

fn spelled_date(word: &str) -> Option<String> {
    let b = word.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (word[..4].parse::<u32>().ok()?, word[5..7].parse::<usize>().ok()?, word[8..].parse::<u32>().ok()?);
    Some(format!("{d} de {} de {y}", MONTH_NAMES.get(m.checked_sub(1)?)?))
}

/// O Perfil fala com a pessoa. O arquivo segue em terceira pessoa, que é como a IA lê.
pub fn to_you(s: &str) -> String {
    let text = s.replace("A pessoa", "Você").replace("a pessoa", "você");
    let mut out = String::with_capacity(text.len());
    for piece in text.split_inclusive(' ') {
        let word = piece.trim_end_matches([' ', ',', '.', ':', ';']);
        match spelled_date(word) {
            Some(d) => {
                out.push_str(&d);
                out.push_str(&piece[word.len()..]);
            }
            None => out.push_str(piece),
        }
    }
    out
}

/// Volta para o arquivo: "você" dito a uma IA seria ela mesma.
pub fn from_you(s: &str) -> String {
    s.replace("Você", "A pessoa").replace("você", "a pessoa")
}

/// Texto vindo de arquivo nunca entra na tela com invisível ou bidi: um nome falsificaria a extensão.
fn shown(s: &str) -> String {
    s.chars()
        .map(|c| if ed::md::is_hidden_char(c) || is_private(c) { format!("⟦U+{:04X}⟧", c as u32) } else { c.to_string() })
        .collect()
}

fn is_private(c: char) -> bool {
    matches!(c, '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{10FFFD}' | '\u{2800}' | '\u{1D173}'..='\u{1D17A}')
}

fn held_reason(code: &str) -> String {
    let parts: Vec<&str> = code
        .split([',', ' '])
        .filter(|p| !p.is_empty())
        .map(|p| match p {
            "instruction_like" => "parece uma ordem para a IA",
            "invisible_text" => "tem texto invisível",
            "header_block" => "tem um bloco escondido no corpo",
            "external" | "external_origin" => "veio de fora",
            "secret" => "parece conter um segredo",
            _ => "conteúdo suspeito",
        })
        .collect();
    let mut seen: Vec<&str> = Vec::new();
    for p in parts {
        if !seen.contains(&p) {
            seen.push(p);
        }
    }
    if seen.is_empty() { "conteúdo suspeito".into() } else { seen.join(", ") }
}

fn title_cased(s: &str) -> String {
    let t = s.replace(['-', '_'], " ");
    let mut c = t.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// Id de um item como as telas o conhecem. O diário grava nota como `note:<caminho>`; na interface é só o caminho.
/// Sem isto, a leitura de uma nota pela IA não acendia no grafo nem contava nas métricas.
pub fn node_id(target: &str) -> &str {
    target.strip_prefix("note:").unwrap_or(target)
}

/// Nome do grupo de quem ainda não tem nota de projeto.
pub const UNREGISTERED_GROUP: &str = "Sem nota de projeto";
const GENERAL_GROUP: &str = "Geral";

/// Como o grafo agrupa cada nó: o grupo dá a cor, o projeto junta os itens dentro do grupo.
struct Grouping {
    region: Vec<u32>,
    groups: Vec<String>,
    project: Vec<u32>,
    projects: Vec<String>,
    hub: Vec<bool>,
    /// Memória → nota do seu projeto: a ligação que prende cada uma ao centro.
    belongs: Vec<(u32, u32)>,
}

/// Grupo e projeto de cada nó. A nota do projeto decide; sem nota, vale o nome que veio da pasta.
/// Nota comum fica no grupo da sua pasta, e quem sobra segue os vizinhos.
fn grouping(g: &core::GraphData, metas: &FxHashMap<String, &core::MemoryMeta>, known: &core::Projects) -> Grouping {
    let n = g.nodes.len();
    let (mut groups, mut projects): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    let (mut group_at, mut project_at): (FxHashMap<String, u32>, FxHashMap<String, u32>) = (FxHashMap::default(), FxHashMap::default());
    fn pick(name: &str, names: &mut Vec<String>, at: &mut FxHashMap<String, u32>) -> u32 {
        *at.entry(name.to_string()).or_insert_with(|| {
            names.push(name.to_string());
            names.len() as u32 - 1
        })
    }
    let mut out = Grouping { region: vec![u32::MAX; n], groups: vec![], project: vec![u32::MAX; n], projects: vec![], hub: vec![false; n], belongs: vec![] };
    let pos: FxHashMap<&str, usize> = g.nodes.iter().enumerate().map(|(i, x)| (x.id.as_str(), i)).collect();
    let group_of = |p: &core::Project| if p.group.trim().is_empty() { "Projetos".to_string() } else { p.group.clone() };
    for (i, node) in g.nodes.iter().enumerate() {
        if let Some(m) = metas.get(&node.id) {
            let Some(tag) = core::tags::project_of(&m.tags) else { continue };
            match known.get(tag) {
                Some(p) => {
                    out.region[i] = pick(&group_of(p), &mut groups, &mut group_at);
                    out.project[i] = pick(&p.title, &mut projects, &mut project_at);
                    if let Some(&hub) = pos.get(p.note.as_str()) {
                        out.belongs.push((i as u32, hub as u32));
                    }
                }
                None => {
                    out.region[i] = pick(UNREGISTERED_GROUP, &mut groups, &mut group_at);
                    out.project[i] = pick(&title_cased(tag), &mut projects, &mut project_at);
                }
            }
        } else if let Some(p) = known.of_note(&node.id) {
            out.region[i] = pick(&group_of(p), &mut groups, &mut group_at);
            out.project[i] = pick(&p.title, &mut projects, &mut project_at);
            out.hub[i] = true;
        } else {
            let parts: Vec<&str> = node.id.split('/').collect();
            if parts.len() > 2 {
                out.region[i] = pick(&title_cased(parts[1]), &mut groups, &mut group_at);
            }
        }
    }
    let edges: Vec<(usize, usize)> = g.edges.iter().filter_map(|e| Some((*pos.get(e.from.as_str())?, *pos.get(e.to.as_str())?))).collect();
    for _ in 0..4 {
        for &(a, b) in &edges {
            if out.region[a] == u32::MAX && out.region[b] != u32::MAX {
                out.region[a] = out.region[b];
            } else if out.region[b] == u32::MAX && out.region[a] != u32::MAX {
                out.region[b] = out.region[a];
            }
        }
    }
    let other = if out.region.contains(&u32::MAX) { pick(GENERAL_GROUP, &mut groups, &mut group_at) } else { 0 };
    out.region.iter_mut().filter(|r| **r == u32::MAX).for_each(|r| *r = other);
    out.groups = groups;
    out.projects = projects;
    out
}

/// Projetos do cofre para a tela: os que têm nota e os que só existem como nome de pasta.
fn project_list(metas: &[core::MemoryMeta], known: &core::Projects) -> Vec<ProjectInfo> {
    let mut count: FxHashMap<String, u32> = FxHashMap::default();
    for m in metas {
        if let Some(p) = core::tags::project_of(&m.tags) {
            *count.entry(known.canon(p)).or_insert(0) += 1;
        }
    }
    let mut list: Vec<ProjectInfo> = known
        .list
        .iter()
        .map(|p| ProjectInfo { slug: p.slug.clone(), title: p.title.clone(), group: p.group.clone(), note: Some(p.note.clone()), memories: count.remove(&p.slug).unwrap_or(0), ephemeral: false })
        .collect();
    let mut loose: Vec<(String, u32)> = count.into_iter().collect();
    loose.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    list.extend(loose.into_iter().map(|(slug, memories)| ProjectInfo { title: title_cased(&slug), ephemeral: core::projects::is_ephemeral(&slug), slug, group: String::new(), note: None, memories }));
    list
}

fn verb(e: &Event) -> Option<(&'static str, u8)> {
    Some(match e.ev {
        EventKind::Wrote if e.before.is_some() => ("Atualizou", 5),
        EventKind::Wrote => ("Adicionou", 1),
        EventKind::Merged => ("Mesclou", 3),
        EventKind::Superseded => ("Substituiu", 3),
        EventKind::Archived => ("Arquivou", 2),
        EventKind::Restored => ("Restaurou", 2),
        EventKind::Promoted => ("Aprovou", 2),
        EventKind::Quarantined => ("Em revisão", 4),
        _ => return None,
    })
}

fn link_options(target: String, project: &str) -> Result<core::connect::Options, String> {
    let project = Path::new(project).canonicalize().map_err(|_| "a pasta do projeto não existe".to_string())?;
    let home = core::util::home_dir().unwrap_or_default();
    Ok(core::connect::Options { target, project, home, global: false, print: false })
}

fn build_model(b: &Brain, generation: &mut u64, last_key: &mut u64) -> core::Result<Model> {
    let notes = b.notes()?;
    let metas = b.memories()?;
    let offset = crate::platform::utc_offset();
    let clock = Clock { today: stats::local_time(&core::util::now_ts(), offset).map_or(0, |t| t.0), offset };
    let mut m = Model { name: b.config().name.clone(), path: b.root().to_string_lossy().into_owned(), ..Default::default() };
    let known = b.projects()?;
    let by_id: FxHashMap<String, &core::MemoryMeta> = metas.iter().map(|x| (x.id.clone(), x)).collect();
    let mut per_note: FxHashMap<&str, u32> = FxHashMap::default();
    for x in &metas {
        if let Some(n) = &x.origin.note {
            *per_note.entry(n.as_str()).or_insert(0) += 1;
        }
    }
    let agenda = b.agenda(&core::AgendaRequest { limit: 40, offset: 0, kinds: vec![] })?;
    let uncovered: FxHashSet<&str> = agenda.items.iter().filter(|i| i.kind == "uncovered" || i.kind == "note").map(|i| i.id.as_str()).collect();
    m.notes = notes
        .iter()
        .map(|n| {
            let parts: Vec<&str> = n.path.split('/').collect();
            let folder = if parts.len() > 2 { title_cased(parts[1]) } else { String::new() };
            Note { path: n.path.clone(), modified: day_text(&n.modified, clock), title: shown(&n.title), folder, bytes: n.size, covered: !uncovered.contains(n.path.as_str()), memories: per_note.get(n.path.as_str()).copied().unwrap_or(0), aliases: n.aliases.clone() }
        })
        .collect();
    let changed: FxHashMap<&str, &str> = notes.iter().map(|n| (n.path.as_str(), n.modified.as_str())).collect();
    m.notes.sort_by(|a, b| changed.get(b.path.as_str()).cmp(&changed.get(a.path.as_str())).then(a.title.cmp(&b.title)));
    m.memories = metas
        .iter()
        .map(|x| Memory {
            id: x.id.clone(),
            hash: x.hash.clone(),
            kind: mem_kind(x.mem_type),
            title: shown(&x.title),
            summary: shown(&x.summary),
            body: String::new(),
            state: state(x.state),
            authority: authority(x.authority),
            origin: origin_label(x.origin.kind).into(),
            client: x.origin.client.clone().unwrap_or_default(),
            session: x.origin.session.clone().unwrap_or_default(),
            quote: x.origin.quote.clone().unwrap_or_default(),
            note: x.origin.note.clone(),
            created: day_text(&x.created, clock),
            tags: x.tags.clone(),
            uses: 0,
            held: x.quarantine_reason.as_deref().map(held_reason),
            project: core::tags::project_of(&x.tags).map(|p| known.canon(p)),
        })
        .collect();
    m.quarantined = metas.iter().filter(|x| x.state == MemoryState::Quarantined).count() as u32;

    let sessions = b.sessions()?;
    let mut g = b.graph()?;
    // Nota entra no grafo como `note:<caminho>`; na interface o id é o caminho.
    for n in &mut g.nodes {
        if let Some(p) = n.id.strip_prefix("note:") {
            n.id = p.to_string();
        }
    }
    for e in &mut g.edges {
        if let Some(p) = e.from.strip_prefix("note:") {
            e.from = p.to_string();
        }
        if let Some(p) = e.to.strip_prefix("note:") {
            e.to = p.to_string();
        }
    }
    let groups = grouping(&g, &by_id, &known);
    m.projects = project_list(&metas, &known);
    let pos: FxHashMap<&str, u32> = g.nodes.iter().enumerate().map(|(i, x)| (x.id.as_str(), i as u32)).collect();
    let nodes = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, x)| GraphNode {
            id: x.id.clone(),
            kind: x.mem_type.map_or(NodeKind::Note, |t| NodeKind::Memory(mem_kind(t))),
            title: if x.state == Some(MemoryState::Quarantined) { "Memória em revisão".into() } else { shown(&x.title) },
            weight: x.weight,
            region: groups.region[i],
            project: groups.project[i],
            hub: groups.hub[i],
        })
        .collect();
    for e in g.edges.iter().filter(|e| e.kind == "wikilink") {
        m.backlinks.entry(e.to.clone()).or_default().push(e.from.clone());
    }
    let mut edges: Vec<(u32, u32, f32)> = g.edges.iter().filter_map(|e| Some((*pos.get(e.from.as_str())?, *pos.get(e.to.as_str())?, e.weight))).collect();
    edges.extend(groups.belongs.iter().map(|&(a, b)| (a, b, 1.5)));
    m.graph = Graph { nodes, edges, regions: groups.groups, projects: groups.projects };

    let events = journal::read_all(b.root());
    let undone: FxHashSet<&str> = events.iter().filter_map(|e| e.undoes.as_deref()).collect();
    let mut by_session: FxHashMap<&str, Vec<&Event>> = FxHashMap::default();
    let mut reads: FxHashMap<&str, u32> = FxHashMap::default();
    for e in &events {
        if verb(e).is_some() {
            by_session.entry(e.session.as_str()).or_default().push(e);
        }
        if e.ev == EventKind::Read {
            *reads.entry(e.session.as_str()).or_insert(0) += 1;
        }
    }
    let mut ordered: Vec<_> = sessions.iter().filter(|s| by_session.contains_key(s.id.as_str()) || reads.contains_key(s.id.as_str())).collect();
    ordered.sort_by(|a, b| b.started.cmp(&a.started));
    for s in ordered.iter().take(80) {
        let changes = by_session
            .get(s.id.as_str())
            .map(|v| {
                v.iter()
                    .rev()
                    .filter_map(|e| {
                        let (word, fallback) = verb(e)?;
                        let target = e.target.clone().unwrap_or_default();
                        // O símbolo é sempre o tipo da memória, como no mapa e na busca.
                        let kind = by_id.get(&target).map_or(fallback, |x| mem_kind(x.mem_type) as u8);
                        // O título daquele momento: a história não se reescreve quando a memória muda.
                        let then = e.after.as_deref().and_then(|t| core::parse_memory(t).ok()).filter(|m| m.quarantine.is_none() && by_id.get(&target).is_none_or(|x| x.state != MemoryState::Quarantined)).map(|m| shown(&m.title));
                        let title = then.or_else(|| by_id.get(&target).map(|x| x.title.clone())).unwrap_or_else(|| e.text.clone().unwrap_or_else(|| "Memória".into()));
                        Some(Change {
                            event: e.id.clone(),
                            verb: word.into(),
                            target,
                            title,
                            kind,
                            reason: e.reason.clone().or_else(|| e.rule.clone()).unwrap_or_default(),
                            before: e.before.as_deref().map(body_of).unwrap_or_default(),
                            after: e.after.as_deref().map(body_of).unwrap_or_default(),
                            undone: undone.contains(e.id.as_str()),
                            undoable: e.ev.is_mutation(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        m.sessions.push(Session { session: s.id.clone(), client: stats::client_label(&s.client), project: s.project.clone().unwrap_or_default(), day: day_text(&s.started, clock), time: hour_text(&s.started, clock), reads: reads.get(s.id.as_str()).copied().unwrap_or(0), changes });
    }
    let mut clients: FxHashMap<&str, &str> = FxHashMap::default();
    for s in sessions.iter().filter(|s| !stats::is_person(&s.client)) {
        let e = clients.entry(s.client.as_str()).or_insert(s.last_event.as_str());
        if s.last_event.as_str() > *e {
            *e = s.last_event.as_str();
        }
    }
    let mut clients: Vec<(&str, &str)> = clients.into_iter().collect();
    clients.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    m.clients = clients.into_iter().map(|(n, t)| Client { name: n.into(), last_seen: format!("{}, {}", day_text(t, clock), hour_text(t, clock)) }).collect();

    m.links = core::connect::connections(b.root())
        .into_iter()
        .map(|c| {
            let state = core::connect::status(b.root(), &c);
            let name = c.project.rsplit('/').next().unwrap_or(&c.project).to_string();
            Link { project: c.project, name, client: c.client, state }
        })
        .collect();

    let home = crate::platform::home_dir();
    m.global_links = ["claude-code", "copilot"].into_iter().filter(|c| core::connect::global_status(b.root(), &home, c) == LinkState::On).map(str::to_string).collect();

    m.profile = b
        .portrait()?
        .groups
        .into_iter()
        .map(|g| ProfileSection {
            title: facet_title(&g.facet),
            lines: g.lines.into_iter().map(|l| ProfileLine { text: to_you(&l.text), memory: l.id, authority: authority(l.authority) }).collect(),
        })
        .collect();
    let mut kinds: Vec<(&String, &usize)> = agenda.remaining.iter().collect();
    kinds.sort_by(|a, b| b.1.cmp(a.1));
    m.agenda = kinds
        .into_iter()
        .filter(|(_, n)| **n > 0)
        .map(|(k, n)| AgendaItem { kind: k.clone(), text: agenda_text(k, *n) })
        .collect();
    m.metrics = stats::compute(&stats::Sources { events: &events, notes: &m.notes, memories: &m.memories, graph: &m.graph, today: clock.today, offset: clock.offset });

    // A geração só muda se o desenho do grafo mudou: leitura não refaz a geometria.
    let mut key = 0xcbf29ce484222325u64;
    for n in &m.graph.nodes {
        for b in n.id.bytes() {
            key = (key ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    for n in &m.graph.nodes {
        key = (key ^ (n.region as u64) << 1 ^ n.project as u64).wrapping_mul(0x100000001b3);
    }
    for name in m.graph.regions.iter().chain(&m.graph.projects) {
        for b in name.bytes() {
            key = (key ^ b as u64).wrapping_mul(0x100000001b3);
        }
    }
    key ^= m.graph.edges.len() as u64;
    if key != *last_key {
        *last_key = key;
        *generation += 1;
    }
    m.generation = *generation;
    Ok(m)
}

/// `project/rota-certa` vira "Rota certa": faceta é formato interno, não texto de tela.
fn facet_title(f: &str) -> String {
    match f.split_once('/') {
        Some((_, rest)) => title_cased(rest),
        None => title_cased(f),
    }
}


fn agenda_text(kind: &str, n: usize) -> String {
    let s = |one: &str, many: &str| if n == 1 { one.to_string() } else { format!("{n} {many}") };
    match kind {
        "uncovered" | "note" | "new" => s("1 nota que a IA nunca leu", "notas que a IA nunca leu"),
        "changed" => s("1 nota mudou depois que a IA leu", "notas mudaram depois que a IA leu"),
        "stale" => s("1 memória envelheceu", "memórias envelheceram"),
        "conflict" | "contradiction" => s("1 contradição entre memórias", "contradições entre memórias"),
        "duplicate" | "near_duplicate" => s("1 par de memórias quase iguais", "pares de memórias quase iguais"),
        "gap" => s("1 pergunta sem resposta no cofre", "perguntas sem resposta no cofre"),
        "note_new" => s("1 nota nova que a IA ainda não leu", "notas novas que a IA ainda não leu"),
        "note_changed" => s("1 nota mudou desde a última leitura", "notas mudaram desde a última leitura"),
        "note_corrected" => s("1 nota com correção sua", "notas com correção sua"),
        "idle" => s("1 memória sem uso há muito tempo", "memórias sem uso há muito tempo"),
        "signal" => s("1 correção sua ainda sem virar memória", "correções suas ainda sem virar memória"),
        "title" => s("1 memória com título que não se explica sozinho", "memórias com título que não se explica sozinho"),
        _ => s("1 pendência", "pendências"),
    }
}

struct Worker {
    brain: Option<Brain>,
    tx: Sender<Msg>,
    wake: Box<dyn Fn() + Send>,
    cursor: core::Cursor,
    generation: u64,
    last_key: u64,
    open: Option<(String, Option<std::time::SystemTime>, u64, String)>,
    dirty: bool,
    closing: bool,
    last_build: Instant,
    build_cost: Duration,
}

impl Worker {
    fn send(&self, m: Msg) {
        let _ = self.tx.send(m);
        (self.wake)();
    }

    fn toast(&self, t: impl Into<String>) {
        self.send(Msg::Toast(t.into()));
    }

    fn open_brain(&mut self, b: Brain) {
        let batch = b.journal_tail(&core::Cursor::default());
        let mut c = batch.map(|x| x.cursor).unwrap_or_default();
        loop {
            match b.journal_tail(&c) {
                Ok(x) if !x.events.is_empty() => c = x.cursor,
                _ => break,
            }
        }
        self.cursor = c;
        self.brain = Some(b);
        self.snapshot();
        if let Some(b) = &self.brain {
            let _ = b.sleep(Duration::from_millis(60));
        }
    }

    fn snapshot(&mut self) {
        self.dirty = false;
        let Some(b) = &self.brain else { return };
        let started = Instant::now();
        let built = build_model(b, &mut self.generation, &mut self.last_key);
        self.build_cost = started.elapsed();
        self.last_build = Instant::now();
        if std::env::var_os("OMNIMIND_TRACE_START").is_some() {
            eprintln!("snapshot built in {:?}", self.build_cost);
        }
        match built {
            Ok(m) => self.send(Msg::Snapshot(Arc::new(m))),
            Err(e) => self.toast(format!("Não consegui ler o cofre: {e}")),
        }
    }

    fn file_stamp(&self, path: &str) -> (Option<std::time::SystemTime>, u64) {
        let Some(b) = &self.brain else { return (None, 0) };
        let m = std::fs::metadata(b.root().join(path)).ok();
        (m.as_ref().and_then(|m| m.modified().ok()), m.map_or(0, |m| m.len()))
    }

    fn poll(&mut self) {
        let Some(b) = &self.brain else { return };
        if let Ok(batch) = b.journal_tail(&self.cursor) {
            self.cursor = batch.cursor;
            let mut mutated = false;
            for e in &batch.events {
                let client = stats::client_label(&e.client);
                match e.ev {
                    EventKind::Read => {
                        if let Some(t) = e.target.as_deref().or(e.path.as_deref()) {
                            self.send(Msg::Live(Live::Read { node: node_id(t).to_string(), client }));
                        }
                    }
                    EventKind::Searched => {
                        let query = e.queries.as_ref().and_then(|q| q.first().cloned()).unwrap_or_default();
                        let found = e.results.as_ref().map_or(0, |r| r.len() as u32);
                        self.send(Msg::Live(Live::Searched { query, client, found }));
                    }
                    EventKind::Session if e.kind.as_deref() == Some("end") => self.send(Msg::Live(Live::SessionEnded)),
                    EventKind::Wrote | EventKind::Merged | EventKind::Superseded => {
                        let target = e.target.clone().unwrap_or_default();
                        let title = e.text.clone().unwrap_or_else(|| "memória nova".into());
                        self.send(Msg::Live(Live::Wrote { node: target, title, client }));
                        mutated = true;
                    }
                    EventKind::Undid => {
                        self.send(Msg::Live(Live::Undid { node: e.target.clone().unwrap_or_default() }));
                        mutated = true;
                    }
                    _ => mutated |= e.ev.is_mutation() || matches!(e.ev, EventKind::Note | EventKind::External),
                }
            }
            self.dirty |= mutated;
        }
        if let Some((path, mtime, size, _)) = self.open.clone() {
            let (m, s) = self.file_stamp(&path);
            if (m, s) != (mtime, size) {
                if let Some(b) = &self.brain {
                    if let Ok(n) = b.read_note(&path) {
                        self.open = Some((path.clone(), m, s, n.meta.hash.clone()));
                        self.send(Msg::External { path, text: n.text, hash: n.meta.hash });
                        self.dirty = true;
                    }
                }
            }
        }
        // IA gravando em rajada: refazer custa, então o ritmo acompanha o custo. No máximo 15% de um núcleo.
        if self.dirty && self.last_build.elapsed() >= self.build_cost * 6 {
            self.snapshot();
        }
    }

    fn handle(&mut self, c: Cmd) -> bool {
        match c {
            Cmd::Refresh => self.snapshot(),
            Cmd::Closing => self.closing = true,
            Cmd::Shutdown => {
                if let Some(b) = &self.brain {
                    let _ = b.flush();
                }
                return false;
            }
            Cmd::CreateBrain(p) => match Brain::init(&p, "Pessoal").or_else(|_| Brain::open(&p)) {
                Ok(b) => {
                    // Só o que a pessoa escolheu na tela vira o brain de amanhã.
                    remember_brain(b.root());
                    self.open_brain(b);
                }
                Err(e) => self.toast(format!("Não consegui criar o cofre: {e}")),
            },
            Cmd::OpenBrain(p) => {
                let p = if p == "example" { example_brain() } else { Some(p) };
                match p.map(|p| Brain::open(&p).or_else(|_| Brain::init(&p, "Exemplo"))) {
                    Some(Ok(b)) => {
                        remember_brain(b.root());
                        self.open_brain(b);
                    }
                    Some(Err(e)) => self.toast(format!("Não consegui abrir: {e}")),
                    None => self.toast("O exemplo não veio com esta instalação."),
                }
            }
            Cmd::Connect { client, project } => {
                let root = self.brain.as_ref().map(|b| b.root().to_path_buf()).unwrap_or_default();
                let brain = root.canonicalize().unwrap_or(root);
                let bin = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join("omnimind")));
                let done = link_options(client, &project).and_then(|o| match bin {
                    Some(b) => core::connect::connect(&o, &b.to_string_lossy(), &brain.to_string_lossy(), &mut |_| {}),
                    None => Err("não achei o programa ao lado do app".into()),
                });
                match done {
                    Ok(_) => self.toast("Ligado neste projeto. Vale a partir da próxima sessão."),
                    Err(e) => self.toast(format!("Não consegui ligar: {e}")),
                }
                self.snapshot();
            }
            Cmd::Disconnect { client, project } => {
                match link_options(client, &project).and_then(|o| core::connect::disconnect(&o, &mut |_| {})) {
                    Ok(_) => self.toast("Desligado neste projeto. O cofre continua igual."),
                    Err(e) => self.toast(format!("Não consegui desligar: {e}")),
                }
                self.snapshot();
            }
            Cmd::ForgetLink { client, project } => {
                if let Some(b) = &self.brain {
                    core::connect::forget(b.root(), &project, &client);
                }
                self.snapshot();
            }
            c => {
                if let Some(b) = self.brain.take() {
                    self.with_brain(&b, c);
                    self.brain = Some(b);
                }
            }
        }
        true
    }

    fn with_brain(&mut self, b: &Brain, c: Cmd) {
        match c {
            Cmd::OpenNote(path) => match b.read_note(&path) {
                Ok(n) => {
                    b.note_opened(&path);
                    let m = std::fs::metadata(b.root().join(&path)).ok();
                    self.open = Some((path.clone(), m.as_ref().and_then(|m| m.modified().ok()), m.map_or(0, |m| m.len()), n.meta.hash.clone()));
                    self.send(Msg::Note { path, text: n.text, hash: n.meta.hash });
                }
                Err(e) => self.toast(format!("Não consegui abrir: {e}")),
            },
            Cmd::SaveNote { path, text, base } => match b.save_note(&path, &text, Some(&base)) {
                Ok(SaveResult::Saved { hash }) => {
                    let m = std::fs::metadata(b.root().join(&path)).ok();
                    self.open = Some((path.clone(), m.as_ref().and_then(|m| m.modified().ok()), m.map_or(0, |m| m.len()), hash.clone()));
                    self.send(Msg::Saved { path, hash });
                    self.dirty = true;
                }
                Ok(SaveResult::Conflict { current_hash, current_text }) => {
                    if self.closing {
                        // Saindo não há quem escolha: as duas versões ficam.
                        let _ = b.save_note(&sibling(b, &path), &text, None);
                    } else {
                        self.send(Msg::Conflict { path, disk: current_text, hash: current_hash });
                    }
                }
                Err(e) => {
                    if self.closing {
                        let _ = b.save_note(&sibling(b, &path), &text, None);
                    }
                    self.send(Msg::SaveFailed { path, reason: e.to_string() });
                }
            },
            Cmd::SaveLeaving { path, text, base } => {
                match b.save_note(&path, &text, Some(&base)) {
                    Ok(SaveResult::Saved { .. }) => {}
                    Ok(SaveResult::Conflict { .. }) | Err(_) => match b.save_note(&sibling(b, &path), &text, None) {
                        Ok(_) => self.toast("Sua versão foi guardada numa nota ao lado."),
                        Err(e) => self.toast(format!("Não consegui guardar: {e}")),
                    },
                }
                self.dirty = true;
            }
            Cmd::KeepBoth { path, text } => {
                match b.save_note(&sibling(b, &path), &text, None) {
                    Ok(_) => self.toast("Sua versão foi guardada numa nota ao lado."),
                    Err(e) => self.toast(format!("Não consegui guardar: {e}")),
                }
                self.dirty = true;
            }
            Cmd::CreateNote(title) => {
                let path = note_path(b, &title);
                match b.save_note(&path, &format!("# {title}\n\n"), None) {
                    Ok(_) => self.handle_open(b, path),
                    Err(e) => self.toast(format!("Não consegui criar: {e}")),
                }
            }
            Cmd::DailyNote => {
                let day = core::util::today();
                let path = format!("notes/daily/{day}.md");
                if !b.root().join(&path).exists() {
                    let _ = b.save_note(&path, &format!("# {day}\n\n"), None);
                }
                self.handle_open(b, path);
            }
            Cmd::RenameNote { path, title } => {
                let to = match path.rsplit_once('/') {
                    Some((dir, _)) => format!("{dir}/{title}.md"),
                    None => format!("{title}.md"),
                };
                match b.rename_note(&path, &to) {
                    Ok(r) => {
                        self.toast(format!("Renomeada. {} links atualizados.", r.links_rewritten));
                        self.handle_open(b, to);
                    }
                    Err(e) => self.toast(format!("Não consegui renomear: {e}")),
                }
            }
            Cmd::DeleteNote(path) => {
                match b.delete_note(&path) {
                    Ok(_) => self.toast("Foi para a lixeira do cofre. Nada se perdeu."),
                    Err(e) => self.toast(format!("Não consegui apagar: {e}")),
                }
                self.open = None;
                self.dirty = true;
            }
            Cmd::LoadMemory(id) => {
                if let Ok(Some(m)) = b.memory(&id) {
                    self.send(Msg::MemoryBody { id, hash: m.meta.hash, body: body_of(&m.revealed) });
                }
            }
            Cmd::Undo(ev) => {
                match b.undo(&ev) {
                    Ok(_) => self.toast("Desfeito."),
                    Err(e) => self.toast(format!("Não dá para desfazer: {e}")),
                }
                self.dirty = true;
            }
            Cmd::UndoSession(s) => {
                match b.undo_session(&s) {
                    Ok(r) => self.toast(format!("Sessão desfeita: {} mudanças.", r.undid_events.len())),
                    Err(e) => self.toast(format!("Não dá para desfazer: {e}")),
                }
                self.dirty = true;
            }
            Cmd::Forget(id) => {
                let hash = b.memory(&id).ok().flatten().map(|m| m.meta.hash);
                match b.apply(CoreChange::Archive { id, base_hash: hash, reason: Some("esquecida pela pessoa".into()) }, Actor::person()) {
                    Ok(a) if !matches!(a.outcome, core::Outcome::Rejected { .. }) => self.toast("Arquivada. Dá para desfazer em Atividade."),
                    Ok(_) => self.toast("Não consegui arquivar: a memória mudou."),
                    Err(e) => self.toast(format!("Não consegui: {e}")),
                }
                self.dirty = true;
            }
            Cmd::Promote { id, hash } => {
                // Recusa volta como resultado, não como erro: sem olhar, a tela diria que aprovou.
                match b.apply(CoreChange::Promote { id, base_hash: Some(hash), reason: None }, Actor::person()) {
                    Ok(a) if !matches!(a.outcome, core::Outcome::Rejected { .. }) => self.toast("Memória aprovada."),
                    _ => self.toast("A memória mudou. Leia de novo antes de aprovar."),
                }
                self.dirty = true;
            }
            // Abrir a frase e sair sem mudar nada não é correção.
            Cmd::SetProject { id, hash, project } => {
                let general = project.is_none();
                match b.apply(CoreChange::SetProject { id, base_hash: Some(hash), project }, Actor::person()) {
                    Ok(a) if !matches!(a.outcome, core::Outcome::Rejected { .. }) => self.toast(if general { "Agora vale em todo projeto." } else { "Projeto da memória alterado." }),
                    _ => self.toast("A memória mudou. Abra de novo antes de mover."),
                }
                self.dirty = true;
            }
            Cmd::MoveProject { from, to } => {
                let moved = b.move_project(&from, to.as_deref(), &Actor::person()).unwrap_or(0);
                self.toast(match (moved, &to) {
                    (0, _) => "Nenhuma memória para mover.".to_string(),
                    (1, None) => "1 memória agora vale em todo projeto.".to_string(),
                    (n, None) => format!("{n} memórias agora valem em todo projeto."),
                    (1, Some(_)) => "1 memória mudou de projeto.".to_string(),
                    (n, Some(_)) => format!("{n} memórias mudaram de projeto."),
                });
                self.dirty = true;
            }
            Cmd::CreateProject { slug, group } => {
                let known = b.projects().unwrap_or_default();
                if known.get(&slug).is_some() {
                    self.toast("Esse projeto já tem nota.");
                    return;
                }
                // A nota nova fica ao lado das outras notas de projeto.
                let folder = known.list.first().and_then(|p| p.note.rsplit_once('/')).map_or("notes/projetos".to_string(), |(dir, _)| dir.to_string());
                let path = format!("{folder}/{slug}.md");
                let text = core::projects::note_template(&title_cased(&slug), &group, None, &[]);
                match b.save_note(&path, &text, None) {
                    Ok(_) => self.toast(format!("Nota do projeto criada em {path}.")),
                    Err(e) => self.toast(format!("Não consegui criar a nota: {e}")),
                }
                self.dirty = true;
            }
            Cmd::Correct { memory: Some(id), text } if b.memory(&id).ok().flatten().is_some_and(|m| to_you(m.memory.summary.trim()) == text || to_you(m.memory.title.trim()) == text) => {}
            Cmd::Correct { memory: Some(id), text } => {
                let hash = b.memory(&id).ok().flatten().map(|m| m.meta.hash);
                match b.apply(CoreChange::CorrectLine { id, base_hash: hash, text: from_you(&text) }, Actor::person()) {
                    Ok(a) if !matches!(a.outcome, core::Outcome::Rejected { .. }) => self.toast("Corrigido. Vale a partir da próxima resposta."),
                    Ok(_) => self.toast("Não consegui corrigir: a memória mudou."),
                    Err(e) => self.toast(format!("Não consegui corrigir: {e}")),
                }
                self.dirty = true;
            }
            Cmd::Search { query, seq } => {
                if query.trim().is_empty() {
                    return;
                }
                let mut q = SearchQuery::person(&query);
                q.limit = 8;
                q.audience = Audience::Person;
                q.include_old = true;
                match b.search(&q) {
                    Ok(r) => {
                        let hits = r
                            .hits
                            .iter()
                            .map(|h| Hit { id: if h.kind == "note" { h.path.clone() } else { h.id.clone() }, title: h.title.clone(), snippet: h.snippet.clone(), kind: h.mem_type.map_or(0, |t| mem_kind(t) as u8), is_note: h.kind == "note" })
                            .collect();
                        let note = format!("Nada em {} notas e {} memórias.", r.coverage.notes, r.coverage.memories);
                        self.send(Msg::Results { seq, hits, note });
                    }
                    Err(e) => self.send(Msg::Results { seq, hits: vec![], note: format!("A busca falhou: {e}") }),
                }
            }
            Cmd::Sleep => {
                let _ = b.sleep(Duration::from_millis(200));
                self.dirty = true;
            }
            _ => {}
        }
    }

    fn handle_open(&mut self, b: &Brain, path: String) {
        self.with_brain(b, Cmd::OpenNote(path));
        self.dirty = true;
    }
}

/// Nota irmã com nome livre: guardar uma versão nunca pode sobrescrever outra.
fn sibling(b: &Brain, path: &str) -> String {
    let stem = path.trim_end_matches(".md");
    let mut n = 1;
    loop {
        let p = if n == 1 { format!("{stem} (minha versão).md") } else { format!("{stem} (minha versão {n}).md") };
        if !b.root().join(&p).exists() {
            return p;
        }
        n += 1;
    }
}

fn note_path(b: &Brain, title: &str) -> String {
    let slug: String = title.to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    let mut n = 0;
    loop {
        let p = if n == 0 { format!("notes/{slug}.md") } else { format!("notes/{slug}-{n}.md") };
        if !b.root().join(&p).exists() {
            return p;
        }
        n += 1;
    }
}

fn example_brain() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    // No macOS o exemplo mora no pacote; no Windows e no Linux, ao lado do executável.
    let dir = exe.parent()?;
    let bundled = [dir.parent()?.join("Resources/example"), dir.join("example")].into_iter().find(|p| p.exists());
    let src = bundled.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/brains/personal"));
    if !src.exists() {
        return None;
    }
    let dst = core::util::home_dir()?.join("Omnimind Exemplo");
    if !dst.exists() {
        copy_dir(&src, &dst).ok()?;
    }
    Some(dst.to_string_lossy().into_owned())
}

fn copy_dir(a: &Path, b: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(b)?;
    for e in std::fs::read_dir(a)? {
        let e = e?;
        let to = b.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &to)?;
        } else if e.file_type()?.is_file() {
            std::fs::copy(e.path(), to)?;
        }
    }
    Ok(())
}

pub fn spawn(rx: Receiver<Cmd>, tx: Sender<Msg>, wake: Box<dyn Fn() + Send>, brain: Option<String>, demo_mode: bool) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("data".into())
        .spawn(move || {
            if demo_mode {
                return run_demo(rx, tx, wake);
            }
            let mut w = Worker { brain: None, tx, wake, cursor: core::Cursor::default(), generation: 0, last_key: 0, open: None, dirty: false, closing: false, last_build: Instant::now(), build_cost: Duration::ZERO };
            let path = brain.or_else(remembered_brain);
            match path.as_deref().map(Brain::open) {
                Some(Ok(b)) => w.open_brain(b),
                _ => w.send(Msg::NoBrain),
            }
            let mut last = Instant::now();
            loop {
                match rx.recv_timeout(POLL) {
                    Ok(c) => {
                        if !w.handle(c) {
                            return;
                        }
                        if w.dirty {
                            w.snapshot();
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                if last.elapsed() >= POLL {
                    last = Instant::now();
                    w.poll();
                }
            }
        })
        .expect("data thread")
}

fn run_demo(rx: Receiver<Cmd>, tx: Sender<Msg>, wake: Box<dyn Fn() + Send>) {
    let _ = tx.send(Msg::Snapshot(demo::shared()));
    wake();
    while let Ok(c) = rx.recv() {
        match c {
            Cmd::OpenNote(path) => {
                let _ = tx.send(Msg::Note { path, text: demo::note_text(), hash: "demo".into() });
            }
            Cmd::SaveNote { path, .. } => {
                let _ = tx.send(Msg::Saved { path, hash: "demo".into() });
            }
            Cmd::Shutdown => return,
            _ => {}
        }
        wake();
    }
}
