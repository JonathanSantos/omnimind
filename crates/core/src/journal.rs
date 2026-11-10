//! Diário: uma linha JSON por evento, `O_APPEND` com uma única escrita. Só anexa; nunca reescreve.

use crate::error::{Error, Result};
use crate::util;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Wrote,
    Merged,
    Superseded,
    Archived,
    Restored,
    Undid,
    Quarantined,
    Promoted,
    Rejected,
    Read,
    Searched,
    Cited,
    Helped,
    Hurt,
    Signal,
    External,
    Covered,
    Session,
    Note,
    Tool,
    #[serde(other)]
    Unknown,
}

impl EventKind {
    /// Muda arquivo de memória e pode ser desfeito.
    pub fn is_mutation(self) -> bool {
        matches!(
            self,
            EventKind::Wrote
                | EventKind::Merged
                | EventKind::Archived
                | EventKind::Restored
                | EventKind::Undid
                | EventKind::Quarantined
                | EventKind::Promoted
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub ts: String,
    pub seq: u64,
    pub session: String,
    pub client: String,
    pub ev: EventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undoes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Para `session`: `start` ou `end`. Para `signal`: `correction`, `request`, `preference`.
    /// Para `note`: `saved`, `renamed`, `deleted`, `restored`. Para `tool`: nome da ferramenta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queries: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

impl Event {
    pub fn new(ev: EventKind) -> Self {
        Event {
            id: String::new(),
            ts: String::new(),
            seq: 0,
            session: String::new(),
            client: String::new(),
            ev,
            target: None,
            path: None,
            before: None,
            after: None,
            before_hash: None,
            after_hash: None,
            reason: None,
            rule: None,
            undoes: None,
            key: None,
            kind: None,
            text: None,
            queries: None,
            results: None,
            status: None,
            cwd: None,
            project: None,
        }
    }
}

/// Sessão vira nome de arquivo: fora de `[A-Za-z0-9_-]{1,64}`, usa hash.
pub fn session_file_name(session: &str) -> String {
    let ok = !session.is_empty()
        && session.len() <= 64
        && session
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        session.to_string()
    } else {
        format!("h{}", util::hash_str(session))
    }
}

pub struct Journal {
    root: PathBuf,
    files: Mutex<HashMap<String, (PathBuf, File)>>,
    seq: AtomicU64,
}

impl Journal {
    pub fn new(root: &Path) -> Self {
        Journal {
            root: root.to_path_buf(),
            files: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
        }
    }

    fn dir(&self) -> PathBuf {
        self.root.join(".omnimind").join("journal")
    }

    fn open_segment(&self, month: &str, name: &str) -> Result<(PathBuf, File, bool)> {
        if !self.root.join("brain.toml").exists() {
            return Err(Error::BrainNotFound("brain.toml missing".into()));
        }
        let path = self.dir().join(month).join(format!("{name}.jsonl"));
        crate::paths::create_parents(&self.root, &path)?;
        let f = OpenOptions::new().create(true).append(true).open(&path)?;
        // Linha cortada por `kill -9` não pode colar na próxima.
        let mut needs_newline = false;
        if let Ok(mut r) = File::open(&path) {
            if let Ok(m) = r.metadata() {
                if m.len() > 0 && r.seek(SeekFrom::End(-1)).is_ok() {
                    let mut b = [0u8; 1];
                    needs_newline = r.read_exact(&mut b).is_ok() && b[0] != b'\n';
                }
            }
        }
        Ok((path, f, needs_newline))
    }

    /// Preenche id, hora e seq, e grava numa única chamada.
    pub fn append(&self, ev: &mut Event) -> Result<()> {
        if ev.id.is_empty() {
            ev.id = util::new_ulid();
        }
        ev.ts = util::now_ts();
        ev.seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let month = ev.ts[..7].to_string();
        let name = session_file_name(&ev.session);
        let line = serde_json::to_string(ev).map_err(|e| Error::Invalid(e.to_string()))?;
        let mut buf = Vec::with_capacity(line.len() + 2);
        let mut guard = self.files.lock().unwrap_or_else(|p| p.into_inner());
        let slot = format!("{month}/{name}");
        let stale = match guard.get(&slot) {
            Some((p, _)) => !p.exists(),
            None => true,
        };
        if stale {
            let (p, f, nl) = self.open_segment(&month, &name)?;
            if nl {
                buf.push(b'\n');
            }
            guard.insert(slot.clone(), (p, f));
        }
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
        let Some((_, file)) = guard.get_mut(&slot) else {
            return Err(Error::Io("segment".into()));
        };
        file.write_all(&buf)?;
        Ok(())
    }
}

/// Segmentos existentes, relativos a `.omnimind/journal`.
pub fn list_segments(root: &Path) -> Vec<String> {
    let dir = root.join(".omnimind").join("journal");
    let mut out = Vec::new();
    let Ok(months) = fs::read_dir(&dir) else {
        return out;
    };
    for m in months.flatten() {
        let Ok(name) = m.file_name().into_string() else {
            continue;
        };
        let Ok(ft) = m.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let Ok(files) = fs::read_dir(m.path()) else {
            continue;
        };
        for f in files.flatten() {
            let Ok(fname) = f.file_name().into_string() else {
                continue;
            };
            if fname.ends_with(".jsonl") {
                out.push(format!("{name}/{fname}"));
            }
        }
    }
    out.sort();
    out
}

#[derive(Debug, Default)]
pub struct SegmentRead {
    pub events: Vec<Event>,
    pub offset: u64,
    pub bad_lines: u64,
    pub partial_tail: bool,
}

/// Lê de `offset` até a última linha completa. Linha cortada no fim não avança o cursor.
pub fn read_segment(path: &Path, offset: u64) -> std::io::Result<SegmentRead> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let mut out = SegmentRead {
        offset,
        ..Default::default()
    };
    if offset >= len {
        return Ok(out);
    }
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = Vec::with_capacity((len - offset) as usize);
    f.read_to_end(&mut buf)?;
    let mut pos = 0usize;
    while let Some(nl) = buf[pos..].iter().position(|b| *b == b'\n') {
        let line = &buf[pos..pos + nl];
        pos += nl + 1;
        if line.is_empty() {
            continue;
        }
        match serde_json::from_slice::<Event>(line) {
            Ok(e) => out.events.push(e),
            Err(_) => out.bad_lines += 1,
        }
    }
    out.offset = offset + pos as u64;
    out.partial_tail = pos < buf.len();
    Ok(out)
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Cursor {
    pub offsets: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JournalBatch {
    pub events: Vec<Event>,
    pub cursor: Cursor,
    pub bad_lines: u64,
    pub partial_tails: u64,
}

fn order_key(e: &Event) -> (String, u64, String) {
    (e.ts.clone(), e.seq, e.id.clone())
}

/// Eventos novos desde `cursor`, unidos por `ts` + `seq`. Para o app seguir ao vivo.
pub fn tail(root: &Path, cursor: &Cursor, limit: usize) -> JournalBatch {
    let mut out = JournalBatch {
        cursor: cursor.clone(),
        ..Default::default()
    };
    let base = root.join(".omnimind").join("journal");
    for seg in list_segments(root) {
        if out.events.len() >= limit {
            break;
        }
        let off = cursor.offsets.get(&seg).copied().unwrap_or(0);
        let Ok(r) = read_segment(&base.join(&seg), off) else {
            continue;
        };
        out.bad_lines += r.bad_lines;
        out.partial_tails += u64::from(r.partial_tail);
        out.cursor.offsets.insert(seg, r.offset);
        out.events.extend(r.events);
    }
    out.events.sort_by_key(order_key);
    out
}

/// Todos os eventos em ordem.
pub fn read_all(root: &Path) -> Vec<Event> {
    tail(root, &Cursor::default(), usize::MAX).events
}

/// Busca um evento por id sem desserializar o resto.
pub fn find_event(root: &Path, id: &str) -> Option<Event> {
    let base = root.join(".omnimind").join("journal");
    let needle = format!("\"id\":\"{id}\"");
    for seg in list_segments(root) {
        let Ok(text) = fs::read(base.join(&seg)) else {
            continue;
        };
        for line in text.split(|b| *b == b'\n') {
            if line.windows(needle.len()).any(|w| w == needle.as_bytes()) {
                if let Ok(e) = serde_json::from_slice::<Event>(line) {
                    if e.id == id {
                        return Some(e);
                    }
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UseStats {
    pub read: u32,
    pub cited: u32,
    pub helped: u32,
    pub hurt: u32,
    /// Índice da última sessão em que a memória foi útil.
    pub last_useful_session: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mutation {
    pub event: String,
    pub ts: String,
    pub session: String,
    pub ev: EventKind,
    pub target: String,
    pub before_hash: Option<String>,
    pub after_hash: Option<String>,
    pub undoes: Option<String>,
}

fn live_path(root: &Path, client: &str, cwd: &str) -> PathBuf {
    let project = crate::brain::project_from_cwd(cwd);
    root.join(".omnimind").join("live").join(format!("{client}--{project}"))
}

/// O hook conhece a sessão do cliente, e o servidor MCP, não: este aviso liga os dois.
pub fn mark_live(root: &Path, client: &str, cwd: Option<&str>, session: &str) {
    let Some(cwd) = cwd else { return };
    let path = live_path(root, client, cwd);
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let _ = fs::write(path, session);
}

/// Só limpa o próprio aviso: outra sessão pode já ter tomado o lugar.
pub fn clear_live(root: &Path, client: &str, cwd: Option<&str>, session: &str) {
    if live_session(root, client, cwd).as_deref() == Some(session) {
        if let Some(cwd) = cwd {
            let _ = fs::remove_file(live_path(root, client, cwd));
        }
    }
}

pub fn live_session(root: &Path, client: &str, cwd: Option<&str>) -> Option<String> {
    let text = fs::read_to_string(live_path(root, client, cwd?)).ok()?;
    let id = text.trim();
    (!id.is_empty()).then(|| id.to_string())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalRec {
    pub id: String,
    pub ts: String,
    pub session: String,
    pub kind: String,
    pub text: String,
    pub simhash: u64,
    pub rejected: bool,
    #[serde(default)]
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub client: String,
    pub project: Option<String>,
    pub cwd: Option<String>,
    pub started: String,
    pub ended: Option<String>,
    pub last_event: String,
    pub events: u32,
    pub new_memories: u32,
    pub signals: u32,
    pub searches: u32,
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchRec {
    pub ts: String,
    pub session: String,
    pub queries: Vec<String>,
    pub results: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadRec {
    pub ts: String,
    pub session: String,
    pub target: String,
}

const MAX_SEARCHES: usize = 5000;

/// Resumo incremental do diário por processo. Reconstruível do zero.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct JournalView {
    pub offsets: BTreeMap<String, u64>,
    pub keys: HashMap<String, String>,
    pub last_hash: HashMap<String, String>,
    /// Hash gravado pelo próprio produto. Mudança externa não entra: é o que prova a autoridade do arquivo.
    pub product_hash: HashMap<String, String>,
    pub merge_events: HashMap<String, String>,
    /// Pares que a pessoa separou ao desfazer uma fusão. O sono não refunde.
    pub blocked_merges: HashSet<String>,
    pub mutations: Vec<Mutation>,
    pub undone: HashSet<String>,
    pub uses: HashMap<String, UseStats>,
    pub covered: HashMap<String, String>,
    pub session_reads: HashMap<String, HashMap<String, String>>,
    pub signals: Vec<SignalRec>,
    pub sessions: BTreeMap<String, SessionInfo>,
    pub searches: Vec<SearchRec>,
    pub reads: Vec<ReadRec>,
    pub bad_lines: u64,
    pub partial_tails: u64,
    pub total_events: u64,
}

impl JournalView {
    /// Lê o que falta. Segmento encolhido ou apagado reconstrói tudo.
    pub fn catch_up(&mut self, root: &Path) {
        let segs = list_segments(root);
        let set: HashSet<&String> = segs.iter().collect();
        let shrank = self.offsets.iter().any(|(s, off)| {
            if !set.contains(s) {
                return true;
            }
            let p = root.join(".omnimind").join("journal").join(s);
            fs::metadata(p).map_or(true, |m| m.len() < *off)
        });
        if shrank {
            *self = JournalView::default();
        }
        let base = root.join(".omnimind").join("journal");
        let mut fresh = Vec::new();
        self.partial_tails = 0;
        for seg in segs {
            let off = self.offsets.get(&seg).copied().unwrap_or(0);
            let p = base.join(&seg);
            let Ok(meta) = fs::metadata(&p) else { continue };
            if meta.len() <= off {
                continue;
            }
            let Ok(r) = read_segment(&p, off) else {
                continue;
            };
            self.bad_lines += r.bad_lines;
            self.partial_tails += u64::from(r.partial_tail);
            self.offsets.insert(seg, r.offset);
            fresh.extend(r.events);
        }
        fresh.sort_by_key(order_key);
        for e in &fresh {
            self.apply(e);
        }
    }

    fn session_entry(&mut self, e: &Event) -> &mut SessionInfo {
        let next = self.sessions.len() as u32;
        let s = self
            .sessions
            .entry(e.session.clone())
            .or_insert_with(|| SessionInfo {
                id: e.session.clone(),
                client: e.client.clone(),
                started: e.ts.clone(),
                index: next,
                ..Default::default()
            });
        s.last_event = e.ts.clone();
        s.events += 1;
        s
    }

    pub fn apply(&mut self, e: &Event) {
        // Abrir uma nota no app não é sessão: só alimenta as métricas de leitura.
        if e.kind.as_deref() == Some("note_opened") {
            return;
        }
        self.total_events += 1;
        let sidx = self.session_entry(e).index;
        match e.ev {
            EventKind::Session => {
                let s = self.session_entry(e);
                s.events -= 1;
                if e.cwd.is_some() {
                    s.cwd = e.cwd.clone();
                }
                if e.project.is_some() {
                    s.project = e.project.clone();
                }
                if e.kind.as_deref() == Some("end") {
                    s.ended = Some(e.ts.clone());
                }
            }
            EventKind::Signal => {
                if let Some(text) = &e.text {
                    self.signals.push(SignalRec {
                        id: e.id.clone(),
                        ts: e.ts.clone(),
                        session: e.session.clone(),
                        kind: e.kind.clone().unwrap_or_default(),
                        text: text.clone(),
                        simhash: crate::simhash::simhash(text),
                        rejected: false,
                        cwd: e.cwd.clone(),
                    });
                }
                self.session_entry(e).signals += 1;
            }
            EventKind::Rejected => {
                if let Some(t) = &e.target {
                    if let Some(s) = self.signals.iter_mut().find(|s| &s.id == t) {
                        s.rejected = true;
                    }
                }
            }
            EventKind::Searched => {
                self.session_entry(e).searches += 1;
                if self.searches.len() >= MAX_SEARCHES {
                    self.searches.drain(..MAX_SEARCHES / 2);
                }
                self.searches.push(SearchRec {
                    ts: e.ts.clone(),
                    session: e.session.clone(),
                    queries: e.queries.clone().unwrap_or_default(),
                    results: e.results.clone().unwrap_or_default(),
                    status: e.status.clone().unwrap_or_default(),
                });
            }
            EventKind::Read => {
                if let Some(t) = &e.target {
                    self.uses.entry(t.clone()).or_default().read += 1;
                    if self.reads.len() >= MAX_SEARCHES {
                        self.reads.drain(..MAX_SEARCHES / 2);
                    }
                    self.reads.push(ReadRec {
                        ts: e.ts.clone(),
                        session: e.session.clone(),
                        target: t.clone(),
                    });
                    if let (Some(p), Some(h)) = (&e.path, &e.after_hash) {
                        self.session_reads
                            .entry(e.session.clone())
                            .or_default()
                            .insert(p.clone(), h.clone());
                    }
                }
            }
            EventKind::Covered => {
                if let (Some(p), Some(h)) = (&e.path, &e.after_hash) {
                    self.covered.insert(p.clone(), h.clone());
                    self.session_reads
                        .entry(e.session.clone())
                        .or_default()
                        .insert(p.clone(), h.clone());
                }
            }
            EventKind::Cited | EventKind::Helped | EventKind::Hurt => {
                if let Some(t) = &e.target {
                    let u = self.uses.entry(t.clone()).or_default();
                    match e.ev {
                        EventKind::Cited => u.cited += 1,
                        EventKind::Helped => {
                            u.helped += 1;
                            u.last_useful_session = sidx;
                        }
                        _ => u.hurt += 1,
                    }
                }
            }
            EventKind::External | EventKind::Note => {
                if let Some(k) = e.target.clone().or_else(|| e.path.clone()) {
                    match &e.after_hash {
                        Some(h) => {
                            self.last_hash.insert(k, h.clone());
                        }
                        None => {
                            self.last_hash.remove(&k);
                        }
                    }
                }
            }
            _ => {}
        }
        if e.ev.is_mutation() {
            if let Some(t) = &e.target {
                match &e.after_hash {
                    Some(h) => {
                        self.last_hash.insert(t.clone(), h.clone());
                    }
                    None => {
                        self.last_hash.remove(t);
                    }
                }
                self.mutations.push(Mutation {
                    event: e.id.clone(),
                    ts: e.ts.clone(),
                    session: e.session.clone(),
                    ev: e.ev,
                    target: t.clone(),
                    before_hash: e.before_hash.clone(),
                    after_hash: e.after_hash.clone(),
                    undoes: e.undoes.clone(),
                });
                match &e.after_hash {
                    Some(h) => {
                        self.product_hash.insert(t.clone(), h.clone());
                    }
                    None => {
                        self.product_hash.remove(t);
                    }
                }
                if let Some(u) = &e.undoes {
                    self.undone.insert(u.clone());
                    if let Some(k) = self.merge_events.get(u).cloned() {
                        self.blocked_merges.insert(k);
                    }
                }
                if let Some(k) = &e.key {
                    if k.starts_with("merge:") {
                        self.merge_events.insert(e.id.clone(), k.clone());
                    } else if e.ev != EventKind::Undid {
                        self.keys.insert(k.clone(), t.clone());
                    }
                }
                if matches!(e.ev, EventKind::Wrote | EventKind::Quarantined)
                    && e.before_hash.is_none()
                {
                    self.session_entry(e).new_memories += 1;
                }
            }
        }
        if e.ev == EventKind::Undid {
            // Idempotência: desfeita a escrita, a chave volta a estar livre.
            if let Some(t) = &e.target {
                if e.after_hash.is_none() {
                    self.keys.retain(|_, v| v != t);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_brain() -> PathBuf {
        let p = std::env::temp_dir().join(format!("vz-journal-{}", util::new_ulid()));
        fs::create_dir_all(p.join(".omnimind")).unwrap();
        fs::write(p.join("brain.toml"), "x").unwrap();
        p
    }

    fn ev(kind: EventKind, session: &str) -> Event {
        let mut e = Event::new(kind);
        e.session = session.into();
        e.client = "t".into();
        e
    }

    #[test]
    fn truncated_tail_ignored_and_next_line_survives() {
        let root = temp_brain();
        let j = Journal::new(&root);
        let mut e = ev(EventKind::Searched, "s1");
        j.append(&mut e).unwrap();
        let seg = list_segments(&root)[0].clone();
        let p = root.join(".omnimind/journal").join(&seg);
        let mut f = OpenOptions::new().append(true).open(&p).unwrap();
        f.write_all(b"{\"id\":\"cortado").unwrap();
        drop(f);
        let view_before = tail(&root, &Cursor::default(), 100);
        assert_eq!(view_before.events.len(), 1);
        assert_eq!(view_before.partial_tails, 1);
        // outro processo abre o mesmo segmento depois do corte
        let j2 = Journal::new(&root);
        let mut e2 = ev(EventKind::Searched, "s1");
        j2.append(&mut e2).unwrap();
        let after = tail(&root, &Cursor::default(), 100);
        assert_eq!(after.events.len(), 2);
        assert_eq!(after.bad_lines, 1);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cursor_advances_only_over_complete_lines() {
        let root = temp_brain();
        let j = Journal::new(&root);
        j.append(&mut ev(EventKind::Searched, "s")).unwrap();
        let b1 = tail(&root, &Cursor::default(), 100);
        assert_eq!(b1.events.len(), 1);
        let b2 = tail(&root, &b1.cursor, 100);
        assert!(b2.events.is_empty());
        j.append(&mut ev(EventKind::Searched, "s")).unwrap();
        assert_eq!(tail(&root, &b2.cursor, 100).events.len(), 1);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn never_recreates_a_vanished_brain() {
        let root = temp_brain();
        let j = Journal::new(&root);
        fs::remove_dir_all(&root).unwrap();
        assert!(j.append(&mut ev(EventKind::Searched, "s")).is_err());
        assert!(!root.exists());
    }

    #[test]
    fn hostile_session_names_become_hashes() {
        assert_eq!(session_file_name("abc-1"), "abc-1");
        assert!(session_file_name("../../x").starts_with('h'));
        assert!(!session_file_name("../../x").contains('/'));
    }

    #[test]
    fn view_rebuilds_when_segment_shrinks() {
        let root = temp_brain();
        let j = Journal::new(&root);
        let mut e = ev(EventKind::Covered, "s");
        e.path = Some("notes/a.md".into());
        e.after_hash = Some("h".into());
        j.append(&mut e).unwrap();
        let mut v = JournalView::default();
        v.catch_up(&root);
        assert_eq!(v.covered.len(), 1);
        fs::remove_dir_all(root.join(".omnimind/journal")).unwrap();
        v.catch_up(&root);
        assert!(v.covered.is_empty());
        fs::remove_dir_all(&root).unwrap();
    }
}
