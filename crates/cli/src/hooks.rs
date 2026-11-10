//! Hooks: subcomandos do binário. Lêem JSON do stdin, só anexam ao diário e nunca bloqueiam o cliente.

use crate::Args;
use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use omnimind_core::journal::{session_file_name, Event, EventKind, Journal};
use omnimind_core::{briefing_light, classify, inspect, strip_pasted, util, SignalKind};

const STDIN_CAP: u64 = 8 * 1024 * 1024;
const SIGNAL_CAP: usize = 2000;

fn read_stdin() -> Value {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::stdin().take(STDIN_CAP).read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    match rx.recv_timeout(Duration::from_millis(250)) {
        Ok(b) => serde_json::from_slice(&b).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn field<'a>(v: &'a Value, snake: &str, camel: &str) -> Option<&'a str> {
    v.get(snake)
        .or_else(|| v.get(camel))
        .and_then(Value::as_str)
}

fn emit_context(event: &str, text: &str, client: &str) {
    let out = if client == "copilot" {
        json!({"additionalContext": text})
    } else {
        json!({"hookSpecificOutput": {"hookEventName": event, "additionalContext": text}})
    };
    println!("{out}");
}

struct Ctx {
    root: PathBuf,
    journal: Journal,
    session: String,
    client: String,
}

impl Ctx {
    fn event(&self, kind: EventKind) -> Event {
        let mut e = Event::new(kind);
        e.session.clone_from(&self.session);
        e.client.clone_from(&self.client);
        e
    }

    fn append(&self, mut e: Event) {
        let _ = self.journal.append(&mut e);
    }
}

/// Últimos `cited` ainda sem desfecho: entre o fim do segmento e o último sinal ou reforço.
fn pending_cited(ctx: &Ctx) -> Vec<String> {
    let name = session_file_name(&ctx.session);
    let month = &util::now_ts()[..7];
    let path = ctx
        .root
        .join(".omnimind/journal")
        .join(month)
        .join(format!("{name}.jsonl"));
    let Ok(meta) = std::fs::metadata(&path) else {
        return Vec::new();
    };
    let Ok(mut f) = std::fs::File::open(&path) else {
        return Vec::new();
    };
    use std::io::{Seek, SeekFrom};
    let tail = meta.len().saturating_sub(256 * 1024);
    if f.seek(SeekFrom::Start(tail)).is_err() {
        return Vec::new();
    }
    let mut text = String::new();
    if f.read_to_string(&mut text).is_err() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for line in text.lines().rev() {
        if line.contains("\"ev\":\"cited\"") {
            if let Ok(e) = serde_json::from_str::<Event>(line) {
                if let Some(t) = e.target {
                    if !out.contains(&t) {
                        out.push(t);
                    }
                }
            }
        } else if line.contains("\"ev\":\"helped\"")
            || line.contains("\"ev\":\"hurt\"")
            || line.contains("\"ev\":\"signal\"")
        {
            break;
        }
    }
    out
}

fn on_prompt(ctx: &Ctx, input: &Value) {
    let Some(prompt) = field(input, "prompt", "prompt") else {
        return;
    };
    let cited = pending_cited(ctx);
    let kind = classify(prompt);
    for id in &cited {
        let mut e = ctx.event(if kind == Some(SignalKind::Correction) {
            EventKind::Hurt
        } else {
            EventKind::Helped
        });
        e.target = Some(id.clone());
        ctx.append(e);
    }
    let Some(kind) = kind else { return };
    let clean = inspect::redact(&strip_pasted(prompt));
    let clean = clean.trim();
    if inspect::find_secret(clean).is_some() {
        return;
    }
    let text: String = clean.chars().take(SIGNAL_CAP).collect();
    let mut e = ctx.event(EventKind::Signal);
    e.kind = Some(kind.as_str().to_string());
    e.text = Some(text);
    e.cwd = field(input, "cwd", "cwd").map(str::to_string);
    ctx.append(e);
    let label = match kind {
        SignalKind::Correction => "correção",
        SignalKind::Preference => "preferência",
        SignalKind::Request => "pedido de registro",
    };
    let who = if omnimind_core::signals::names_the_person(prompt) {
        " A mensagem diz quem a pessoa é: guarde isso numa memória só para isso, com a frase literal em quote."
    } else {
        ""
    };
    let text = format!(
        "Omnimind, {}: esta mensagem da pessoa tem forma de {label} e ficou registrada em literal no diário. A tool memory_write guarda a versão destilada, com a frase literal em quote; a tool memory_search mostra o que já existe sobre o tema.{who}",
        &util::now_ts()[..10]
    );
    emit_context("UserPromptSubmit", &text, &ctx.client);
}

fn extract_ids(msg: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tok in msg.split(|c: char| !c.is_ascii_alphanumeric()) {
        if util::is_ulid(tok) && !out.iter().any(|x| x == tok) {
            out.push(tok.to_string());
        }
    }
    out
}

fn on_stop(ctx: &Ctx, input: &Value) {
    let Some(msg) = field(input, "last_assistant_message", "lastAssistantMessage") else {
        return;
    };
    for id in extract_ids(msg).into_iter().take(32) {
        let mut e = ctx.event(EventKind::Cited);
        e.target = Some(id);
        ctx.append(e);
    }
}

fn on_post_tool(ctx: &Ctx, input: &Value) {
    let Some(name) = field(input, "tool_name", "toolName") else {
        return;
    };
    let args = input.get("tool_input").or_else(|| input.get("toolArgs"));
    let path = args.and_then(|a| a.get("file_path").or_else(|| a.get("path"))).and_then(Value::as_str);
    // Só entra no diário a ferramenta que mexeu num arquivo do cofre: é o que interessa auditar.
    // Cada `Bash` e cada `Read` do projeto da pessoa enchiam o diário sem servir a ninguém.
    let Some(path) = path.filter(|p| touches_vault(&ctx.root, p)) else {
        return;
    };
    let mut e = ctx.event(EventKind::Tool);
    // Só o nome e o caminho. Saída de ferramenta nunca entra no diário.
    e.kind = Some(name.chars().take(64).collect());
    e.path = Some(inspect::redact(path).chars().take(300).collect());
    ctx.append(e);
}

/// O caminho aponta para dentro da pasta do cofre?
fn touches_vault(root: &Path, path: &str) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let p = Path::new(path);
    // O arquivo pode nem existir mais: compara pelo que der para resolver, e pelo caminho como veio.
    let resolved = p.canonicalize().or_else(|_| p.parent().map_or(Err(()), |d| d.canonicalize().map_err(|_| ()))).ok();
    p.is_absolute() && (p.starts_with(&root) || resolved.is_some_and(|r| r.starts_with(&root)))
}

fn session_mark(ctx: &Ctx, input: &Value, kind: &str) {
    let mut e = ctx.event(EventKind::Session);
    e.kind = Some(kind.to_string());
    e.cwd = field(input, "cwd", "cwd").map(str::to_string);
    e.project = e.cwd.as_deref().map(omnimind_core::brain::project_from_cwd);
    ctx.append(e);
}

fn on_session_start(ctx: &Ctx, input: &Value) {
    session_mark(ctx, input, "start");
    let cwd = field(input, "cwd", "cwd");
    omnimind_core::journal::mark_live(&ctx.root, &ctx.client, cwd, &ctx.session);
    match briefing_light(Path::new(&ctx.root), cwd, &ctx.client) {
        Ok(b) => emit_context("SessionStart", &b.text, &ctx.client),
        Err(e) => eprintln!("omnimind hook: {e}"),
    }
}

/// Sempre sai com 0. Pânico, brain ausente ou JSON ruim não chegam ao cliente.
pub fn run(args: &Args) -> i32 {
    std::panic::set_hook(Box::new(|_| {
        std::process::exit(0);
    }));
    let event = args
        .positional
        .first()
        .map_or("", String::as_str)
        .to_string();
    let input = read_stdin();
    let Some(root) = args.brain_path() else {
        return 0;
    };
    if !root.join("brain.toml").is_file() {
        return 0;
    }
    let session =
        field(&input, "session_id", "sessionId").map_or_else(|| "hook".to_string(), str::to_string);
    let client = args
        .flag("client")
        .map(str::to_string)
        .or_else(|| std::env::var("OMNIMIND_CLIENT").ok())
        .and_then(|c| omnimind_core::normalize_client(&c).ok())
        .unwrap_or_else(|| "claude-code".to_string());
    let ctx = Ctx {
        journal: Journal::new(&root),
        root,
        session,
        client,
    };
    match event.as_str() {
        "session-start" => on_session_start(&ctx, &input),
        "prompt" => on_prompt(&ctx, &input),
        "post-tool" => on_post_tool(&ctx, &input),
        "pre-compact" => session_mark(&ctx, &input, "compact"),
        "stop" => on_stop(&ctx, &input),
        "session-end" => {
            session_mark(&ctx, &input, "end");
            let cwd = field(&input, "cwd", "cwd");
            omnimind_core::journal::clear_live(&ctx.root, &ctx.client, cwd, &ctx.session);
        }
        _ => {}
    }
    0
}
