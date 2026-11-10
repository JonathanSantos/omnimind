//! Comandos de linha. Tudo lido de disco passa por `safe` antes de ir ao terminal.

use crate::Args;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use omnimind_core::{
    inspect, Actor, AgendaRequest, Brain, BrainOptions, Change, MemoryDraft, MemoryType,
    OriginKind, Outcome, ReadRequest, SearchQuery,
};

/// Escapa controle e bidi: campo de uma linha não forja outra linha.
/// Título curto: a primeira frase. Sem uma, o texto até 100 caracteres, sem cortar palavra.
fn short_title(text: &str) -> String {
    let text = text.trim();
    let first = text
        .find(['.', ';', ':', '\n'])
        .map_or(text, |i| text[..i].trim_end());
    if (12..=60).contains(&first.chars().count()) {
        return first.to_string();
    }
    if text.chars().count() <= 100 {
        return text.to_string();
    }
    let cut: String = text.chars().take(100).collect();
    match cut.rfind(' ') {
        Some(i) => cut[..i].trim_end_matches([',', ' ']).to_string(),
        None => cut,
    }
}

pub fn safe(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in inspect::reveal(s).chars() {
        if c == '\n' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn open(args: &Args) -> Result<Brain, omnimind_core::Error> {
    let path = args.brain_path().ok_or_else(|| {
        omnimind_core::Error::BrainNotFound("no brain: use --brain or OMNIMIND_BRAIN".into())
    })?;
    Brain::open_with(
        path,
        BrainOptions {
            client: Some("person".into()),
            session: args.flag("session").map(str::to_string),
            ..Default::default()
        },
    )
}

fn print_json(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

fn fail(e: &omnimind_core::Error, args: &Args) -> i32 {
    if args.has("json") {
        print_json(&json!({"error": e.code(), "message": e.to_string()}));
    } else {
        eprintln!("{}", human_error(e));
    }
    1
}

/// Mensagem para quem está no terminal. A forma de máquina continua em `--json`.
fn human_error(e: &omnimind_core::Error) -> String {
    use omnimind_core::Error as E;
    match e {
        E::BrainNotFound(_) => "Cofre não encontrado. Use --brain PASTA, OMNIMIND_BRAIN ou rode dentro da pasta do cofre.".into(),
        E::AlreadyExists(p) => format!("Já existe: {}.", safe(p)),
        E::InvalidPath { path, .. } => format!("Caminho recusado: {}.", safe(path)),
        E::NotFound(p) => format!("Não encontrado: {}.", safe(p)),
        E::Busy => "Outro processo está gravando. Tente de novo em instantes.".into(),
        E::Io(k) => format!("Erro de disco: {}.", safe(k)),
        E::Config(m) => format!("brain.toml com problema: {}.", safe(m)),
        E::ClientNotAllowed(c) => format!("Cliente não permitido: {}.", safe(c)),
        E::Invalid(m) => match m.strip_prefix("usage: ") {
            Some(u) => format!("Uso: {}", safe(u)),
            None => format!("Pedido inválido: {}.", safe(m)),
        },
        E::RenameCapturesLinks { notes, .. } => {
            format!("O nome novo capturaria links de {} notas. Escolha outro nome.", notes.len())
        }
        E::UndoRefused { reason, chain } => {
            if chain.is_empty() {
                format!("Não deu para desfazer: {}.", safe(reason))
            } else {
                format!("Não deu para desfazer: a memória mudou depois. Desfaça antes: {}.", chain.join(", "))
            }
        }
        E::ExportRefused(m) => format!("Exportação recusada: {}.", safe(m)),
    }
}

fn outcome_line(a: &omnimind_core::Applied) -> String {
    match &a.outcome {
        Outcome::Written => "Gravada.".to_string(),
        Outcome::Merged { .. } => "Igual a uma memória que já existe: juntei as duas.".to_string(),
        Outcome::Superseded { ids } => format!("Gravada, no lugar de {} memórias.", ids.len()),
        Outcome::Quarantined { reason } => format!("Retida para você revisar: {}.", safe(reason)),
        Outcome::Rejected { reason, .. } => format!("Não gravada: {}.", safe(reason)),
        Outcome::Unchanged => "Já estava gravada, nada mudou.".to_string(),
    }
}

pub fn run(cmd: &str, args: &Args) -> i32 {
    match dispatch(cmd, args) {
        Ok(code) => code,
        Err(e) => fail(&e, args),
    }
}

fn dispatch(cmd: &str, args: &Args) -> Result<i32, omnimind_core::Error> {
    let json_out = args.has("json");
    match cmd {
        "init" => {
            let path = args
                .positional
                .first()
                .ok_or_else(|| omnimind_core::Error::Invalid("usage: omnimind init PATH".into()))?;
            let name = args.flag("name").map_or_else(
                || {
                    Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("brain")
                        .to_string()
                },
                str::to_string,
            );
            let b = Brain::init(path, &name)?;
            if json_out {
                print_json(
                    &json!({"id": b.config().id, "name": b.config().name, "path": b.root()}),
                );
            } else {
                println!(
                    "Cofre {} criado em {}.",
                    safe(&b.config().name),
                    b.root().display()
                );
                println!(
                    "Próximo passo: coloque suas notas em {}/notes e conecte um projeto:",
                    b.root().display()
                );
                println!(
                    "  omnimind connect claude-code --brain {} --project PASTA_DO_PROJETO",
                    b.root().display()
                );
            }
            Ok(0)
        }
        "doctor" => {
            let b = open(args)?;
            let h = b.health()?;
            let v = json!({
                "brain": b.config().name,
                "writes": "reads files; appends journal events only for changes it finds unlogged",
                "health": h,
            });
            if json_out {
                print_json(&v);
            } else {
                println!("cofre: {}", safe(&b.config().name));
                println!("escreve: só anexa ao diário mudanças que achar sem registro");
                println!(
                    "notas: {}  memórias: {}  cobertas: {}",
                    h.notes, h.memories, h.notes_covered
                );
                println!("estados: {:?}", h.states);
                println!(
                    "sem leitura: {}  symlinks recusados: {}",
                    h.unread, h.refused_symlinks
                );
                println!(
                    "diário: {} eventos, {} linhas ruins, {} cortadas",
                    h.journal_events, h.journal_bad_lines, h.journal_partial_tails
                );
                println!(
                    "links quebrados: {}  contradições: {}  sinais abertos: {}",
                    h.dangling_links, h.contradictions, h.open_signals
                );
            }
            b.maybe_snapshot()?;
            Ok(0)
        }
        "lint" => {
            let b = open(args)?;
            let issues = b.lint()?;
            let errors = issues.iter().filter(|i| i.severity == "error").count();
            if json_out {
                print_json(&json!({"issues": issues, "errors": errors}));
            } else {
                for i in &issues {
                    println!(
                        "{:7} {:20} {} {}",
                        i.severity,
                        i.kind,
                        safe(&i.target),
                        safe(&i.detail)
                    );
                }
                println!("{} avisos, {errors} erros", issues.len());
            }
            Ok(i32::from(errors > 0))
        }
        "search" => {
            let b = open(args)?;
            let q = args.positional.join(" ");
            if q.trim().is_empty() {
                return Err(omnimind_core::Error::Invalid(
                    "usage: omnimind search QUERY".into(),
                ));
            }
            let mut sq = SearchQuery::person(&q);
            sq.limit = args
                .flag("limit")
                .and_then(|l| l.parse().ok())
                .unwrap_or(10);
            sq.include_old = args.has("old");
            if let Some(t) = args.flag("type") {
                sq.types = vec![t.to_string()];
            }
            let r = b.search(&sq)?;
            if json_out {
                print_json(&serde_json::to_value(&r).unwrap_or_default());
            } else {
                for h in &r.hits {
                    let state = h.state.map_or("note", |s| s.as_str());
                    println!("{:6.2} {:11} {}  {}", h.score, state, h.id, safe(&h.title));
                    println!("       {}", safe(&h.snippet));
                }
                println!(
                    "situação: {}  total: {}  cobertura: {}",
                    r.status.as_str(),
                    r.total,
                    json!(r.coverage)
                );
            }
            b.maybe_snapshot()?;
            Ok(0)
        }
        "read" => {
            let b = open(args)?;
            let target = args
                .positional
                .first()
                .ok_or_else(|| omnimind_core::Error::Invalid("usage: omnimind read ID".into()))?;
            let r = b.read_item(&ReadRequest {
                target: target.clone(),
                include_old: true,
                ..Default::default()
            })?;
            if json_out {
                print_json(&serde_json::to_value(&r).unwrap_or_default());
            } else if r.status == "ok" {
                println!("{}\n", safe(&r.title));
                println!("{}", inspect::reveal(&r.text));
            } else {
                println!("{}", r.status);
                return Ok(1);
            }
            Ok(0)
        }
        "remember" => {
            let b = open(args)?;
            let text = args.positional.join(" ");
            if text.trim().is_empty() {
                return Err(omnimind_core::Error::Invalid(
                    "usage: omnimind remember TEXT".into(),
                ));
            }
            let mem_type = match args.flag("type") {
                Some(t) => Some(
                    MemoryType::parse(t)
                        .ok_or_else(|| omnimind_core::Error::Invalid("unknown type".into()))?,
                ),
                // "Prefiro..." é preferência mesmo sem --type: é a mesma regra dos hooks.
                None => Some(match omnimind_core::classify(&text) {
                    Some(omnimind_core::SignalKind::Preference) => MemoryType::Preference,
                    _ => MemoryType::Concept,
                }),
            };
            let title = args
                .flag("title")
                .map_or_else(|| short_title(&text), str::to_string);
            let tags = args
                .flag("tags")
                .map(|t| {
                    t.split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let draft = MemoryDraft {
                key: args.flag("key").map(str::to_string),
                mem_type,
                title,
                summary: text.chars().take(380).collect(),
                body: text.clone(),
                tags,
                origin_kind: Some(OriginKind::Request),
                quote: Some(text.chars().take(380).collect()),
                ..Default::default()
            };
            let a = b.apply(
                Change::Write(draft),
                Actor::person().with_session(b.session()),
            )?;
            let stored = a.is_stored() || matches!(a.outcome, Outcome::Quarantined { .. });
            if json_out {
                print_json(&serde_json::to_value(&a).unwrap_or_default());
            } else {
                println!("{}", outcome_line(&a));
                if let Some(id) = &a.id {
                    println!("id: {id}");
                }
                for w in &a.warnings {
                    println!("aviso: {}", safe(w));
                }
            }
            b.maybe_snapshot()?;
            Ok(i32::from(!stored))
        }
        "undo" => {
            let b = open(args)?;
            let r = if let Some(s) = args.flag("session") {
                b.undo_session(s)?
            } else {
                let id = args.positional.first().ok_or_else(|| {
                    omnimind_core::Error::Invalid("usage: omnimind undo EVENT".into())
                })?;
                b.undo(id)?
            };
            if json_out {
                print_json(&serde_json::to_value(&r).unwrap_or_default());
            } else {
                println!("Desfeito: {}", r.undone.join(", "));
            }
            Ok(0)
        }
        "log" => {
            let b = open(args)?;
            let batch = b.journal_tail(&omnimind_core::Cursor::default())?;
            let limit = args
                .flag("limit")
                .and_then(|l| l.parse().ok())
                .unwrap_or(40usize);
            let start = batch.events.len().saturating_sub(limit);
            let events = &batch.events[start..];
            if json_out {
                print_json(
                    &json!({"events": events, "bad_lines": batch.bad_lines, "partial_tails": batch.partial_tails}),
                );
            } else {
                for e in events {
                    let what = e.target.as_deref().or(e.path.as_deref()).unwrap_or("");
                    let ev = serde_json::to_value(e.ev)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_string))
                        .unwrap_or_default();
                    println!(
                        "{} {} {:11} {} {}",
                        e.ts,
                        e.id,
                        ev,
                        safe(what),
                        safe(e.rule.as_deref().or(e.kind.as_deref()).unwrap_or(""))
                    );
                }
            }
            Ok(0)
        }
        "agenda" => {
            let b = open(args)?;
            let a = b.agenda(&AgendaRequest {
                limit: args
                    .flag("limit")
                    .and_then(|l| l.parse().ok())
                    .unwrap_or(20),
                ..Default::default()
            })?;
            if json_out {
                print_json(&serde_json::to_value(&a).unwrap_or_default());
            } else {
                for i in &a.items {
                    println!("{:15} {}  {}", i.kind, i.id, safe(&i.title));
                }
                println!(
                    "total: {}  notas cobertas: {}/{}",
                    a.total, a.notes_covered, a.notes_total
                );
            }
            Ok(0)
        }
        "sleep" => {
            let b = open(args)?;
            let ms = args
                .flag("budget-ms")
                .and_then(|l| l.parse().ok())
                .unwrap_or(1000u64);
            let r = b.sleep(Duration::from_millis(ms))?;
            if json_out {
                print_json(&serde_json::to_value(&r).unwrap_or_default());
            } else {
                println!(
                    "fundidas {}  aliases {}  promovidas {}  restantes {}  sessão {}",
                    r.merged, r.aliases, r.promoted, r.remaining, r.session
                );
            }
            Ok(0)
        }
        "projects" => {
            let b = open(args)?;
            let known = b.projects()?;
            let plan = b.project_plan()?;
            let mut count: std::collections::BTreeMap<String, usize> = Default::default();
            for m in b.memories()? {
                if let Some(p) = omnimind_core::tags::project_of(&m.tags) {
                    *count.entry(known.canon(p)).or_default() += 1;
                }
            }
            // `--move PROJETO [--to OUTRO]`: leva as memórias de um projeto para outro. Sem `--to`, viram gerais.
            if let Some(from) = args.flag("move") {
                let to = args.flag("to");
                let n = b.move_project(from, to, &Actor::person())?;
                let dest = to.map_or("gerais".to_string(), |t| format!("do projeto {}", known.title(t)));
                println!("{n} memórias de {} agora são {dest}. Cada mudança está no diário e pode ser desfeita.", safe(from));
                return Ok(0);
            }
            let apply = args.has("apply");
            let mut moved = 0;
            if apply {
                // Quem roda a CLI é a pessoa: é ela que muda o projeto de uma memória.
                for m in &plan {
                    let done = b.apply(Change::SetProject { id: m.id.clone(), base_hash: Some(m.hash.clone()), project: m.to.clone() }, Actor::person())?;
                    moved += (done.outcome == Outcome::Written) as usize;
                }
            }
            if json_out {
                print_json(&serde_json::json!({"projects": known.list, "memories": count, "plan": plan, "applied": apply, "moved": moved}));
                return Ok(0);
            }
            println!("Com nota ({}):", known.list.len());
            for p in &known.list {
                let n = count.remove(&p.slug).unwrap_or(0);
                if n > 0 {
                    println!("  {} [{}] · {} memórias", safe(&p.title), safe(&p.group), n);
                }
            }
            println!("Sem nota ({}):", count.len());
            for (slug, n) in &count {
                let kind = if omnimind_core::projects::is_ephemeral(slug) { "pasta descartável" } else { "ainda sem nota" };
                println!("  {} · {} memórias · {}", safe(slug), n, kind);
            }
            println!("{} ({}):", if apply { "Mudanças aplicadas" } else { "Mudanças que as regras fariam" }, plan.len());
            for m in &plan {
                let to = m.to.as_deref().map_or("geral".to_string(), |t| known.title(t));
                // Mesmo projeto dos dois lados: a memória tinha duas tags de projeto, e a segunda sai.
                if m.from == m.to {
                    println!("  {} (sai a segunda tag de projeto) · {} [{}]", safe(&to), safe(&m.title), m.id);
                } else {
                    println!("  {} -> {} · {} [{}]", m.from.as_deref().unwrap_or("geral"), safe(&to), safe(&m.title), m.id);
                }
            }
            if apply {
                println!("{moved} memórias mudaram. Cada mudança está no diário e pode ser desfeita.");
            } else if !plan.is_empty() {
                println!("Nada foi alterado. Rode de novo com --apply para aplicar.");
            }
            Ok(0)
        }
        "portrait" => {
            let b = open(args)?;
            let p = b.portrait()?;
            if json_out {
                print_json(&serde_json::to_value(&p).unwrap_or_default());
            } else {
                for g in &p.groups {
                    println!("{}", safe(&g.facet));
                    for l in &g.lines {
                        println!(
                            "  - {} ({}, {}) [{}]",
                            safe(&l.text),
                            l.date,
                            l.authority.as_str(),
                            l.id
                        );
                    }
                }
            }
            Ok(0)
        }
        "export" => {
            let b = open(args)?;
            let out = args.flag("out").ok_or_else(|| {
                omnimind_core::Error::Invalid("usage: omnimind export --out DIR".into())
            })?;
            let r = b.export(Path::new(out))?;
            if json_out {
                print_json(&serde_json::to_value(&r).unwrap_or_default());
            } else {
                println!(
                    "Exportadas {} notas, {} memórias e {} segmentos do diário.",
                    r.notes, r.memories, r.journal_segments
                );
            }
            Ok(0)
        }
        "import-markdown" => {
            let b = open(args)?;
            let src = args.positional.first().ok_or_else(|| {
                omnimind_core::Error::Invalid("usage: omnimind import-markdown SRC".into())
            })?;
            let r = b.import_markdown(Path::new(src))?;
            b.flush()?;
            if json_out {
                print_json(&serde_json::to_value(&r).unwrap_or_default());
            } else {
                println!(
                    "Importadas {} notas, {} memórias e {} preferências; puladas {}.",
                    r.notes, r.memories, r.preferences, r.skipped
                );
                println!("resultados: {:?}", r.outcomes);
            }
            Ok(0)
        }
        other => {
            eprintln!("Comando desconhecido: {}. Rode omnimind help.", safe(other));
            Ok(2)
        }
    }
}
