//! `Brain`: abertura, estado por processo, reconciliação por `stat`, instantâneo e notas.

use crate::analyzer::{Analyzer, ANALYZER_VERSION};
use crate::config::BrainConfig;
use crate::error::{Error, Result};
use crate::index::{put_u32, DocFields, Index, Reader};
use crate::inspect;
use crate::journal::{self, Cursor, Event, EventKind, Journal, JournalBatch, JournalView};
use crate::model::*;
use crate::note::{self, wikilinks, LinkEntry, Resolver};
use crate::paths;
use crate::util;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, UNIX_EPOCH};

const SCAN_INTERVAL: Duration = Duration::from_millis(250);
const MISSING_GRACE: Duration = Duration::from_millis(300);
const RACY_NS: i64 = 2_000_000_000;
const RACY_PER_SCAN: usize = 128;
const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(30);
const LOCK_TIMEOUT: Duration = Duration::from_secs(20);
const SNAPSHOT_MAGIC: &[u8; 8] = b"VZSNAP1\n";
// 3: cada nota guarda a declaração de projeto que faz.
const SNAPSHOT_FORMAT: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FileSig {
    pub size: u64,
    pub mtime_ns: i64,
    pub ctime_ns: i64,
    pub ino: u64,
}

fn sig_of(m: &fs::Metadata) -> FileSig {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        FileSig {
            size: m.len(),
            mtime_ns: m.mtime() * 1_000_000_000 + m.mtime_nsec(),
            ctime_ns: m.ctime() * 1_000_000_000 + m.ctime_nsec(),
            ino: m.ino(),
        }
    }
    #[cfg(not(unix))]
    {
        let mtime = m
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_nanos() as i64);
        FileSig {
            size: m.len(),
            mtime_ns: mtime,
            ctime_ns: 0,
            ino: 0,
        }
    }
}

fn now_ns() -> i64 {
    std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as i64)
}

pub(crate) fn sig_for(m: &fs::Metadata) -> FileSig {
    sig_of(m)
}

pub(crate) fn now_nanos() -> i64 {
    now_ns()
}

fn racy(sig: &FileSig, read_at_ns: i64) -> bool {
    (read_at_ns - sig.mtime_ns.max(sig.ctime_ns)).abs() <= RACY_NS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NoteRec {
    pub path: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub hash: String,
    pub sig: FileSig,
    pub read_at_ns: i64,
    /// Grupo, pastas e apelidos, quando a nota descreve um projeto.
    #[serde(default)]
    pub project: Option<crate::projects::Decl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MemRec {
    pub memory: Memory,
    pub fp: crate::simhash::Fingerprint,
    pub path: String,
    pub hash: String,
    pub sig: FileSig,
    pub read_at_ns: i64,
    /// Motivo achado no conteúdo. `quarantine` é o efetivo, depois de checar promoção e diário.
    pub suspect: Option<String>,
    pub promotion_ok: bool,
    pub verified: bool,
    pub quarantine: Option<String>,
    pub quote_broken: bool,
}

impl MemRec {
    /// `person` no arquivo só vale com evento do produto para este hash.
    pub fn effective_authority(&self) -> Authority {
        if self.memory.authority == Authority::Person && !self.verified {
            Authority::Agent
        } else {
            self.memory.authority
        }
    }

    pub fn flags(&self) -> Vec<String> {
        if self.memory.authority == Authority::Person && !self.verified {
            vec!["unverified_authority".to_string()]
        } else {
            Vec::new()
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct InvalidRec {
    pub reason: String,
    pub sig: FileSig,
}

#[derive(Serialize, Deserialize, Default)]
struct SnapshotMeta {
    brain_id: String,
    notes: Vec<NoteRec>,
    mems: Vec<MemRec>,
    invalid: BTreeMap<String, InvalidRec>,
    view: JournalView,
}

#[derive(Serialize)]
struct SnapshotOut<'a> {
    brain_id: &'a str,
    notes: Vec<&'a NoteRec>,
    mems: Vec<&'a MemRec>,
    invalid: &'a BTreeMap<String, InvalidRec>,
    view: &'a JournalView,
}

pub(crate) struct State {
    pub loaded: bool,
    pub baseline: bool,
    pub index: Index,
    pub analyzer: Analyzer,
    pub notes: HashMap<String, NoteRec>,
    pub mems: HashMap<String, MemRec>,
    pub path_ids: HashMap<String, String>,
    pub invalid: BTreeMap<String, InvalidRec>,
    pub unread: BTreeMap<String, String>,
    pub superseded_by: HashMap<String, String>,
    pub view: JournalView,
    pub pending: Vec<(String, FileSig)>,
    pub missing: HashMap<String, Instant>,
    pub pending_external: Vec<Event>,
    pub refused_symlinks: usize,
    pub last_scan: Option<Instant>,
    pub last_view: Option<Instant>,
    pub gen_seen: u64,
    pub dirty: bool,
    pub snapshot_saved: Option<Instant>,
    pub reading: HashMap<(String, String), (String, usize)>,
    /// Montado na primeira necessidade e mantido por nota; caro de refazer com 5.000 notas.
    pub resolver: Option<Resolver>,
}

impl State {
    /// Projetos que as notas declaram. Barato: só percorre o que já está em memória.
    pub fn projects(&self) -> crate::projects::Projects {
        crate::projects::Projects::build(self.notes.values().filter_map(|n| n.project.as_ref().map(|d| (n.path.as_str(), n.title.as_str(), d))))
    }

    fn new() -> Self {
        State {
            loaded: false,
            baseline: false,
            index: Index::new(),
            analyzer: Analyzer::new(),
            notes: HashMap::new(),
            mems: HashMap::new(),
            path_ids: HashMap::new(),
            invalid: BTreeMap::new(),
            unread: BTreeMap::new(),
            superseded_by: HashMap::new(),
            view: JournalView::default(),
            pending: Vec::new(),
            missing: HashMap::new(),
            pending_external: Vec::new(),
            refused_symlinks: 0,
            last_scan: None,
            last_view: None,
            gen_seen: u64::MAX,
            dirty: false,
            snapshot_saved: None,
            reading: HashMap::new(),
            resolver: None,
        }
    }

    pub fn ensure_resolver(&mut self) {
        if self.resolver.is_none() {
            self.resolver = Some(self.resolver());
        }
    }

    fn resolver_remove(&mut self, n: &NoteRec) {
        if let Some(r) = &mut self.resolver {
            r.remove(&LinkEntry {
                path: &n.path,
                title: &n.title,
                aliases: &n.aliases,
            });
        }
    }

    fn resolver_add(&mut self, path: &str) {
        if let (Some(r), Some(n)) = (&mut self.resolver, self.notes.get(path)) {
            r.add(&LinkEntry {
                path: &n.path,
                title: &n.title,
                aliases: &n.aliases,
            });
        }
    }

    /// Estado derivado, nunca gravado.
    pub fn state_of(&self, rec: &MemRec, today_micros: i64) -> MemoryState {
        if rec.quarantine.is_some() {
            return MemoryState::Quarantined;
        }
        if rec.memory.archived.is_some() {
            return MemoryState::Archived;
        }
        if self.superseded_by.contains_key(&rec.memory.id) {
            return MemoryState::Superseded;
        }
        if let Some(e) = &rec.memory.expires {
            if util::parse_ts(e).is_some_and(|t| t < today_micros) {
                return MemoryState::Expired;
            }
        }
        if self.is_stale(rec) {
            return MemoryState::Stale;
        }
        MemoryState::Active
    }

    pub fn is_stale(&self, rec: &MemRec) -> bool {
        if rec.quote_broken {
            return true;
        }
        match (&rec.memory.origin.note, &rec.memory.origin.hash) {
            (Some(n), Some(h)) => self.notes.get(n).is_none_or(|r| &r.hash != h),
            (Some(n), None) => !self.notes.contains_key(n),
            _ => false,
        }
    }

    /// `superseded` só vale se quem substitui não está em quarentena, arquivada ou vencida.
    pub fn recompute_superseded(&mut self, today_micros: i64) {
        // Quarentena e autoridade dependem do diário, que pode andar depois da leitura do arquivo.
        let State { mems, view, .. } = self;
        for rec in mems.values_mut() {
            rec.verified = view.product_hash.get(&rec.memory.id) == Some(&rec.hash);
            rec.quarantine = if rec.promotion_ok && rec.verified {
                None
            } else {
                rec.suspect.clone()
            };
        }
        let mut map = HashMap::new();
        for rec in self.mems.values() {
            if rec.memory.supersedes.is_empty()
                || rec.quarantine.is_some()
                || rec.memory.archived.is_some()
            {
                continue;
            }
            if let Some(e) = &rec.memory.expires {
                if util::parse_ts(e).is_some_and(|t| t < today_micros) {
                    continue;
                }
            }
            for old in &rec.memory.supersedes {
                if old != &rec.memory.id {
                    map.insert(old.clone(), rec.memory.id.clone());
                }
            }
        }
        self.superseded_by = map;
    }

    pub fn meta_of(&self, rec: &MemRec, now: i64) -> MemoryMeta {
        let m = &rec.memory;
        MemoryMeta {
            id: m.id.clone(),
            mem_type: m.mem_type,
            title: m.title.clone(),
            summary: m.summary.clone(),
            tags: m.tags.clone(),
            authority: rec.effective_authority(),
            flags: rec.flags(),
            state: self.state_of(rec, now),
            origin: m.origin.clone(),
            created: m.created.clone(),
            updated: m.updated.clone(),
            path: rec.path.clone(),
            hash: rec.hash.clone(),
            supersedes: m.supersedes.clone(),
            superseded_by: self.superseded_by.get(&m.id).cloned(),
            quarantine_reason: rec.quarantine.clone(),
        }
    }

    pub fn resolver(&self) -> Resolver {
        Resolver::build(self.notes.values().map(|n| LinkEntry {
            path: &n.path,
            title: &n.title,
            aliases: &n.aliases,
        }))
    }

    pub fn note_meta(n: &NoteRec) -> NoteMeta {
        NoteMeta {
            path: n.path.clone(),
            title: n.title.clone(),
            aliases: n.aliases.clone(),
            tags: n.tags.clone(),
            links: n.links.clone(),
            hash: n.hash.clone(),
            size: n.sig.size,
            modified: util::format_ts_seconds(n.sig.mtime_ns / 1000),
            flags: note_flags(&n.path, &n.title),
        }
    }
}

/// Nome de arquivo e título também são texto de terceiros.
pub(crate) fn note_flags(path: &str, title: &str) -> Vec<String> {
    let mut flags = Vec::new();
    if inspect::has_invisible(path) || inspect::has_invisible(title) {
        flags.push("invisible_name".to_string());
    }
    let name = path.rsplit('/').next().unwrap_or(path);
    let name = name.strip_suffix(".md").unwrap_or(name);
    if inspect::instruction_like(title).is_some() || inspect::instruction_like(name).is_some() {
        flags.push("instruction_like".to_string());
    }
    flags
}

/// Texto suspeito em memória já gravada. Roda no carregamento: arquivo editado por fora também passa aqui.
pub(crate) fn content_fingerprint(m: &Memory) -> String {
    let mut s = String::new();
    for p in [&m.title, &m.summary, &m.body] {
        s.push_str(p);
        s.push('\u{0}');
    }
    for a in m.aliases.iter().chain(m.tags.iter()) {
        s.push_str(a);
        s.push('\u{0}');
    }
    util::hash_str(&s)
}

pub(crate) fn quarantine_reason(m: &Memory) -> Option<String> {
    let mut f = inspect::Findings::default();
    for field in [&m.title, &m.summary, &m.body] {
        f.merge(inspect::inspect(field));
    }
    for field in &m.aliases {
        f.merge(inspect::inspect(field));
    }
    for field in &m.tags {
        f.merge(inspect::inspect_tag(field));
    }
    for field in [
        &m.origin.quote,
        &m.origin.note,
        &m.origin.client,
        &m.origin.session,
    ]
    .into_iter()
    .flatten()
    {
        f.merge(inspect::inspect(field));
    }
    if let Some(q) = &m.quarantine {
        return Some(q.clone());
    }
    if m.authority == Authority::External {
        return Some("external_authority".into());
    }
    if f.secret.is_some() {
        return Some("secret".into());
    }
    if f.invisible {
        return Some("invisible_text".into());
    }
    if f.disguised {
        return Some("disguised_text".into());
    }
    // Vale para `authority: person` também: o campo está no arquivo, e o arquivo pode ser editado.
    if f.header_block {
        return Some("header_block".into());
    }
    if f.instruction.is_some() {
        return Some("instruction_like".into());
    }
    None
}

/// Promoção vale para aquele texto. Quem confere se foi o produto é `recompute_superseded`.
pub(crate) fn promotion_matches(m: &Memory) -> bool {
    m.promoted_hash.as_deref() == Some(content_fingerprint(m).as_str())
}

/// Monta o registro. Segredo sai da RAM e do instantâneo; o arquivo fica como está.
pub(crate) fn make_rec(
    mut m: Memory,
    rel: &str,
    hash: String,
    sig: FileSig,
    read_at: i64,
    quote_broken: bool,
) -> MemRec {
    let suspect = quarantine_reason(&m);
    let promotion_ok = promotion_matches(&m);
    let secret = [&m.title, &m.summary, &m.body]
        .into_iter()
        .chain(m.aliases.iter())
        .chain(m.origin.quote.iter())
        .any(|f| inspect::find_secret(f).is_some());
    if secret {
        m.title = inspect::redact(&m.title);
        m.summary = inspect::redact(&m.summary);
        m.body = inspect::redact(&m.body);
        m.aliases = m.aliases.iter().map(|a| inspect::redact(a)).collect();
        m.origin.quote = m.origin.quote.as_deref().map(inspect::redact);
    }
    MemRec {
        fp: crate::simhash::Fingerprint::of(&m.title, &m.summary, &m.body),
        path: rel.to_string(),
        hash,
        sig,
        read_at_ns: read_at,
        quarantine: suspect.clone(),
        suspect,
        promotion_ok,
        verified: false,
        quote_broken,
        memory: m,
    }
}

/// `id hash` por linha, último vence. Permite ao hook provar autoridade sem ler o diário.
pub(crate) fn read_ledger(root: &Path) -> HashMap<String, String> {
    let Ok(text) = fs::read_to_string(root.join(".omnimind").join("ledger")) else {
        return HashMap::new();
    };
    text.lines()
        .filter_map(|l| l.split_once(' '))
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct BrainOptions {
    pub client: Option<String>,
    pub session: Option<String>,
    pub cwd: Option<String>,
    /// Não carrega nada em `open`; o trabalho acontece nas primeiras consultas ou em `reconcile_slice`.
    pub lazy: bool,
}

pub(crate) struct Inner {
    pub root: PathBuf,
    pub config: BrainConfig,
    pub journal: Journal,
    pub client: String,
    pub session: String,
    pub cwd: Option<String>,
    pub state: Mutex<State>,
    pub no_index: bool,
}

/// Ponto de entrada do núcleo. `Clone + Send + Sync`; clones compartilham o mesmo estado.
#[derive(Clone)]
pub struct Brain {
    pub(crate) inner: Arc<Inner>,
    client_override: Option<Arc<String>>,
    session_override: Option<Arc<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReconcileReport {
    pub changed: usize,
    pub removed: usize,
    pub pending: usize,
    pub scanned: usize,
    pub unread: usize,
    pub ms: u64,
}

pub(crate) struct WriteLock {
    _file: File,
}

fn gitignore_text() -> &'static str {
    ".omnimind/\n"
}

/// Cofre criado quando o produto se chamava Várzea: a pasta interna muda de nome uma única vez.
fn migrate_private(root: &Path, dir: &Path) -> Result<()> {
    let old = root.join(".varzea");
    let is_real_dir = fs::symlink_metadata(&old).is_ok_and(|m| m.is_dir());
    if !is_real_dir || fs::symlink_metadata(dir).is_ok() {
        return Ok(());
    }
    // App, servidor e hooks podem abrir juntos: quem perde a corrida acha a pasta já movida.
    if let Err(e) = fs::rename(&old, dir) {
        if fs::symlink_metadata(dir).is_err() {
            return Err(e.into());
        }
    }
    let gi = root.join(".gitignore");
    if let Ok(text) = fs::read_to_string(&gi) {
        if text.contains(".varzea") {
            let _ = fs::write(&gi, text.replace(".varzea", ".omnimind"));
        }
    }
    Ok(())
}

/// Cria `.omnimind/` e a pasta temporária, recusando symlink.
fn ensure_private(root: &Path) -> Result<PathBuf> {
    let dir = root.join(".omnimind");
    migrate_private(root, &dir)?;
    match fs::symlink_metadata(&dir) {
        Ok(m) if m.file_type().is_symlink() => {
            return Err(Error::InvalidPath {
                path: ".omnimind".into(),
                reason: "symlink refused".into(),
            })
        }
        Ok(_) => {}
        Err(_) => match fs::create_dir(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        },
    }
    let tmp = dir.join("tmp");
    if !tmp.exists() {
        let _ = fs::create_dir(&tmp);
    }
    Ok(dir)
}

impl Brain {
    /// Cria um brain novo. Falha com `AlreadyExists` se já houver `brain.toml`.
    pub fn init(path: impl AsRef<Path>, name: &str) -> Result<Brain> {
        let root = path.as_ref();
        if !root.exists() {
            let parent = root
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            if !parent.exists() {
                return Err(Error::Invalid("parent directory does not exist".into()));
            }
            match fs::create_dir(root) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }
        }
        let root = root.canonicalize()?;
        let cfg = BrainConfig::new(util::new_ulid(), name.to_string(), util::now_ts_seconds());
        // `create_new` elege exatamente um vencedor em `init` concorrente.
        let toml = root.join("brain.toml");
        match OpenOptions::new().write(true).create_new(true).open(&toml) {
            Ok(mut f) => {
                f.write_all(cfg.render().as_bytes())?;
                f.sync_all()?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(Error::AlreadyExists("brain.toml".into()));
            }
            Err(e) => return Err(e.into()),
        }
        for d in ["notes", "memories"] {
            match fs::create_dir(root.join(d)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            }
        }
        for t in MemoryType::ALL {
            let _ = fs::create_dir(root.join("memories").join(t.as_str()));
        }
        ensure_private(&root)?;
        let gi = root.join(".gitignore");
        match fs::read_to_string(&gi) {
            Ok(mut text) => {
                if !text.lines().any(|l| l.trim() == ".omnimind/") {
                    if !text.ends_with('\n') {
                        text.push('\n');
                    }
                    text.push_str(gitignore_text());
                    fs::write(&gi, text)?;
                }
            }
            Err(_) => fs::write(&gi, gitignore_text())?,
        }
        Brain::open(&root)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Brain> {
        Brain::open_with(path, BrainOptions::default())
    }

    pub fn open_with(path: impl AsRef<Path>, opts: BrainOptions) -> Result<Brain> {
        let root = path.as_ref();
        let toml = root.join("brain.toml");
        if !toml.is_file() {
            return Err(Error::BrainNotFound("brain.toml missing".into()));
        }
        let root = root.canonicalize()?;
        let config = BrainConfig::parse(&fs::read_to_string(root.join("brain.toml"))?)?;
        ensure_private(&root)?;
        let session = opts.session.clone().unwrap_or_else(util::new_ulid);
        let brain = Brain {
            client_override: None,
            session_override: None,
            inner: Arc::new(Inner {
                journal: Journal::new(&root),
                root,
                config,
                client: opts.client.clone().unwrap_or_else(|| "person".to_string()),
                session,
                cwd: opts.cwd.clone(),
                state: Mutex::new(State::new()),
                no_index: std::env::var_os("OMNIMIND_TEST_NO_INDEX").is_some(),
            }),
        };
        if !opts.lazy {
            let mut st = brain.lock_state();
            brain.refresh(&mut st, None, true)?;
        }
        Ok(brain)
    }

    pub fn root(&self) -> &Path {
        &self.inner.root
    }

    pub fn config(&self) -> &BrainConfig {
        &self.inner.config
    }

    pub fn session(&self) -> &str {
        self.session_override.as_deref().map_or(&self.inner.session, String::as_str)
    }

    pub fn client(&self) -> &str {
        &self.inner.client
    }

    /// Mesmo brain e estado, com outro cliente no diário.
    pub fn for_client(&self, client: &str) -> Brain {
        let mut b = self.clone();
        b.client_override = Some(Arc::new(client.to_string()));
        b
    }

    /// A mesma visão do cofre, gravando na sessão que o cliente já abriu.
    pub fn for_session(&self, session: &str) -> Brain {
        let mut b = self.clone();
        b.session_override = Some(Arc::new(session.to_string()));
        b
    }

    /// Cobertura atual, sem varrer.
    pub fn coverage_now(&self) -> crate::search::Coverage {
        let st = self.lock_state();
        self.coverage(&st)
    }

    pub(crate) fn lock_state(&self) -> MutexGuard<'_, State> {
        self.inner.state.lock().unwrap_or_else(|p| p.into_inner())
    }

    fn private(&self) -> PathBuf {
        self.inner.root.join(".omnimind")
    }

    pub(crate) fn tmp_dir(&self) -> Result<PathBuf> {
        Ok(ensure_private(&self.inner.root)?.join("tmp"))
    }

    fn check_alive(&self) -> Result<()> {
        if !self.inner.root.join("brain.toml").is_file() {
            return Err(Error::BrainNotFound("brain.toml missing".into()));
        }
        Ok(())
    }

    /// Trava de escrita. Morre com o processo.
    pub(crate) fn write_lock(&self) -> Result<WriteLock> {
        self.check_alive()?;
        let dir = ensure_private(&self.inner.root)?;
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(dir.join("lock"))?;
        let start = Instant::now();
        let mut wait = 1u64;
        loop {
            match file.try_lock() {
                Ok(()) => break,
                Err(std::fs::TryLockError::WouldBlock) => {}
                Err(std::fs::TryLockError::Error(e)) => return Err(e.into()),
            }
            if start.elapsed() > LOCK_TIMEOUT {
                return Err(Error::Busy);
            }
            std::thread::sleep(Duration::from_millis(wait));
            wait = (wait + 1).min(8);
        }
        let lock = WriteLock { _file: file };
        self.resume_plans()?;
        self.clean_tmp();
        Ok(lock)
    }

    fn try_write_lock(&self) -> Option<WriteLock> {
        let dir = ensure_private(&self.inner.root).ok()?;
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(dir.join("lock"))
            .ok()?;
        file.try_lock().ok()?;
        Some(WriteLock { _file: file })
    }

    fn clean_tmp(&self) {
        let dir = self.private().join("tmp");
        let Ok(rd) = fs::read_dir(dir) else { return };
        let cutoff = std::time::SystemTime::now() - Duration::from_secs(120);
        for e in rd.flatten() {
            if let Ok(m) = e.metadata() {
                if m.modified().is_ok_and(|t| t < cutoff) {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
    }

    pub(crate) fn generation_len(&self) -> u64 {
        fs::metadata(self.private().join("generation")).map_or(0, |m| m.len())
    }

    /// Aviso barato para os outros processos: mudou algo, varra já. Perdê-lo só custa até 250 ms.
    pub(crate) fn bump_generation(&self, st: &mut State) {
        let before = self.generation_len();
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.private().join("generation"))
        {
            let _ = f.write_all(b".");
        }
        if before == st.gen_seen {
            st.gen_seen = before + 1;
        }
    }

    pub(crate) fn event(&self, kind: EventKind, actor: Option<&Actor>) -> Event {
        let mut e = Event::new(kind);
        e.session = actor
            .and_then(|a| a.session.clone())
            .unwrap_or_else(|| self.session().to_string());
        e.client = match actor {
            Some(a) => a.client.clone(),
            None => self
                .client_override
                .as_ref()
                .map_or_else(|| self.inner.client.clone(), |c| (**c).clone()),
        };
        e
    }

    pub(crate) fn append(&self, e: &mut Event) -> Result<()> {
        self.inner.journal.append(e)
    }

    /// Registra início ou fim de sessão, com cliente e projeto.
    pub fn record_session(&self, kind: &str, cwd: Option<&str>) -> Result<()> {
        let mut e = self.event(EventKind::Session, None);
        e.kind = Some(kind.to_string());
        e.cwd = cwd.map(str::to_string).or_else(|| self.inner.cwd.clone());
        e.project = e.cwd.as_deref().map(project_from_cwd);
        self.append(&mut e)
    }

    // Reconciliação

    fn load_snapshot(&self, st: &mut State) -> bool {
        let Ok(data) = fs::read(self.private().join("snapshot.bin")) else {
            return false;
        };
        let Some((meta, index)) = decode_snapshot(&data) else {
            return false;
        };
        if meta.brain_id != self.inner.config.id {
            return false;
        }
        st.index = index;
        st.notes = meta
            .notes
            .into_iter()
            .map(|n| (n.path.clone(), n))
            .collect();
        st.path_ids = meta
            .mems
            .iter()
            .map(|m| (m.path.clone(), m.memory.id.clone()))
            .collect();
        st.mems = meta
            .mems
            .into_iter()
            .map(|m| (m.memory.id.clone(), m))
            .collect();
        st.invalid = meta.invalid;
        st.view = meta.view;
        true
    }

    /// Carrega se preciso e varre quando o aviso mudou ou passaram 250 ms.
    pub(crate) fn refresh(
        &self,
        st: &mut State,
        budget: Option<Duration>,
        force: bool,
    ) -> Result<ReconcileReport> {
        self.check_alive()?;
        if !st.loaded {
            st.loaded = true;
            st.baseline = self.load_snapshot(st);
            st.view.catch_up(&self.inner.root);
            st.last_view = Some(Instant::now());
        }
        let gen = self.generation_len();
        let due = st.last_scan.is_none_or(|t| t.elapsed() >= SCAN_INTERVAL);
        let changed = gen != st.gen_seen;
        if !st.pending.is_empty() && !changed && !due {
            // Continua de onde parou, sem refazer o passeio pela árvore.
            let mut report = ReconcileReport::default();
            self.drain(st, budget, &mut report);
            return Ok(report);
        }
        if !(force || due || changed || !st.missing.is_empty()) {
            return Ok(ReconcileReport::default());
        }
        st.gen_seen = gen;
        let report = self.scan(st, budget);
        if st.last_view.is_none_or(|t| t.elapsed() >= SCAN_INTERVAL) || force {
            st.view.catch_up(&self.inner.root);
            st.last_view = Some(Instant::now());
            st.recompute_superseded(util::now_micros());
        }
        self.flush_externals(st);
        Ok(report)
    }

    fn walk(
        &self,
        dir: &Path,
        rel: &str,
        out: &mut Vec<(String, FileSig)>,
        refused: &mut usize,
        unread: &mut BTreeMap<String, String>,
    ) {
        let Ok(rd) = fs::read_dir(dir) else {
            if !rel.is_empty() {
                unread.insert(rel.to_string(), "unreadable directory".into());
            }
            return;
        };
        for e in rd.flatten() {
            let name = e.file_name();
            let Some(name) = name.to_str() else {
                *refused += 1;
                continue;
            };
            if name.starts_with('.') || name == "node_modules" {
                continue;
            }
            let child = if rel.is_empty() {
                name.to_string()
            } else {
                format!("{rel}/{name}")
            };
            let Ok(meta) = e.path().symlink_metadata() else {
                continue;
            };
            let ft = meta.file_type();
            if ft.is_symlink() {
                // Symlink nunca é seguido; dentro ou fora do brain.
                *refused += 1;
            } else if ft.is_dir() {
                self.walk(&e.path(), &child, out, refused, unread);
            } else if ft.is_file() && paths::is_note_name(name) {
                out.push((child, sig_of(&meta)));
            }
        }
    }

    /// Processa a fila dentro do orçamento, que só começa aqui: o passeio pela árvore não consome prazo.
    fn drain(
        &self,
        st: &mut State,
        budget: Option<Duration>,
        report: &mut ReconcileReport,
    ) -> bool {
        let deadline = budget.map(|b| Instant::now() + b);
        let queue = std::mem::take(&mut st.pending);
        let mut any = false;
        let mut iter = queue.into_iter();
        let mut leftover = Vec::new();
        for (done, (rel, sig)) in iter.by_ref().enumerate() {
            // Ao menos um arquivo por fatia: nada morre de fome sob carga.
            if done > 0 && deadline.is_some_and(|d| Instant::now() >= d) {
                leftover.push((rel, sig));
                break;
            }
            match self.ingest_file(st, &rel, sig) {
                Ok(true) => {
                    report.changed += 1;
                    any = true;
                }
                Ok(false) => {}
                Err(Error::Io(k)) if k == std::io::ErrorKind::NotFound.to_string() => {}
                Err(e) => {
                    st.unread.insert(rel, e.code().to_string());
                }
            }
        }
        leftover.extend(iter);
        st.pending = leftover;
        report.pending = st.pending.len();
        any
    }

    fn scan(&self, st: &mut State, budget: Option<Duration>) -> ReconcileReport {
        let started = Instant::now();
        let root = self.inner.root.clone();
        let mut found = Vec::with_capacity(st.notes.len() + st.mems.len() + 16);
        let mut refused = 0;
        let mut unread = BTreeMap::new();
        self.walk(&root, "", &mut found, &mut refused, &mut unread);
        st.refused_symlinks = refused;
        // Ilegível continua contado até voltar a ser lido; a varredura de pastas repõe o resto.
        for (k, v) in std::mem::take(&mut st.unread) {
            if k.ends_with(".md") || k.ends_with(".markdown") {
                unread.entry(k).or_insert(v);
            }
        }
        st.unread = unread;
        let mut report = ReconcileReport {
            scanned: found.len(),
            ..Default::default()
        };

        // Memórias primeiro: se o prazo acabar, o que importa já está de pé.
        found.sort_by_key(|(p, _)| !p.starts_with("memories/"));
        let mut seen = std::collections::HashSet::with_capacity(found.len());
        let mut queue = Vec::new();
        let mut racy_queue = Vec::new();
        for (rel, sig) in found {
            let known = if rel.starts_with("memories/") {
                st.path_ids
                    .get(&rel)
                    .and_then(|id| st.mems.get(id))
                    .map(|m| (m.sig, m.read_at_ns))
                    .or_else(|| st.invalid.get(&rel).map(|i| (i.sig, i64::MAX)))
            } else {
                st.notes.get(&rel).map(|n| (n.sig, n.read_at_ns))
            };
            st.missing.remove(&rel);
            seen.insert(rel.clone());
            match known {
                Some((s, _)) if s != sig => queue.push((rel, sig)),
                // Conferência da regra racy: não é indexação pendente, e tem teto por varredura.
                Some((s, read_at)) => {
                    if read_at != i64::MAX && racy(&s, read_at) && racy_queue.len() < RACY_PER_SCAN
                    {
                        racy_queue.push((rel, sig));
                    }
                }
                None => queue.push((rel, sig)),
            }
        }

        // Sumiu: segura 300 ms, porque editores gravam por rename.
        // Primeira varredura do processo: o instantâneo é velho, não há editor no meio de um rename.
        let first_scan = st.last_scan.is_none();
        let mut gone: Vec<String> = Vec::new();
        for path in st
            .notes
            .keys()
            .chain(st.path_ids.keys())
            .chain(st.invalid.keys())
        {
            if !seen.contains(path) {
                gone.push(path.clone());
            }
        }
        for path in gone {
            let t = *st.missing.entry(path.clone()).or_insert_with(Instant::now);
            if first_scan || t.elapsed() >= MISSING_GRACE {
                st.missing.remove(&path);
                self.forget_path(st, &path, true);
                report.removed += 1;
                st.dirty = true;
            }
        }

        let today = util::now_micros();
        st.pending = queue;
        let mut any = self.drain(st, budget, &mut report);
        if st.pending.is_empty() {
            let deadline = budget.map(|b| Instant::now() + b);
            for (rel, sig) in racy_queue {
                if deadline.is_some_and(|d| Instant::now() >= d) {
                    break;
                }
                if let Ok(true) = self.ingest_file(st, &rel, sig) {
                    report.changed += 1;
                    any = true;
                }
            }
        }
        // Sempre: depende de arquivos e do relógio, e o instantâneo não guarda derivados.
        st.recompute_superseded(today);
        if any || report.removed > 0 {
            st.dirty = true;
        }
        if st.pending.is_empty() {
            st.baseline = true;
            st.analyzer.clear_cache();
            st.index.clear_word_cache();
        }
        st.last_scan = Some(Instant::now());
        report.pending = st.pending.len();
        report.unread = st.unread.len() + st.invalid.len();
        report.ms = started.elapsed().as_millis() as u64;
        report
    }

    /// Lê e indexa. `Ok(true)` quando o conteúdo mudou.
    fn ingest_file(&self, st: &mut State, rel: &str, sig: FileSig) -> Result<bool> {
        let full = self.inner.root.join(rel);
        let read_at = now_ns();
        let bytes = fs::read(&full)?;
        let hash = util::hash_bytes(&bytes);
        let sig = fs::symlink_metadata(&full).map_or(sig, |m| sig_of(&m));
        let text = String::from_utf8_lossy(&bytes);
        if rel.starts_with("memories/") {
            self.ingest_memory(st, rel, &text, hash, sig, read_at)
        } else {
            self.ingest_note(st, rel, &text, hash, sig, read_at)
        }
    }

    fn ingest_note(
        &self,
        st: &mut State,
        rel: &str,
        text: &str,
        hash: String,
        sig: FileSig,
        read_at: i64,
    ) -> Result<bool> {
        if let Some(n) = st.notes.get_mut(rel) {
            if n.hash == hash {
                n.sig = sig;
                n.read_at_ns = read_at;
                return Ok(false);
            }
        }
        let is_new = !st.notes.contains_key(rel);
        if let Some(old) = st.notes.get(rel).cloned() {
            st.resolver_remove(&old);
        }
        // Segredo em nota nunca entra no índice nem no instantâneo. O arquivo fica como está.
        let redacted;
        let text = if inspect::find_secret(text).is_some() {
            redacted = inspect::redact(text);
            &redacted
        } else {
            text
        };
        let p = note::parse_note(rel, text);
        if !self.inner.no_index {
            let body = &text[p.body_offset..];
            let f = DocFields {
                title: &p.title,
                aliases: &p.aliases,
                file: rel
                    .rsplit('/')
                    .next()
                    .unwrap_or(rel)
                    .trim_end_matches(".md"),
                tags: &p.tags,
                summary: &p.summary,
                body,
            };
            let State {
                index, analyzer, ..
            } = st;
            index.add(&format!("note:{rel}"), &f, analyzer);
        }
        st.notes.insert(
            rel.to_string(),
            NoteRec {
                path: rel.to_string(),
                title: p.title,
                aliases: p.aliases,
                tags: p.tags,
                links: p.links,
                hash: hash.clone(),
                sig,
                read_at_ns: read_at,
                project: crate::projects::declare(text),
            },
        );
        st.resolver_add(rel);
        if st.baseline && st.view.last_hash.get(rel) != Some(&hash) {
            let mut e = self.event(EventKind::External, None);
            e.path = Some(rel.to_string());
            e.kind = Some(if is_new { "created" } else { "modified" }.to_string());
            e.after_hash = Some(hash);
            st.pending_external.push(e);
        }
        Ok(true)
    }

    pub(crate) fn index_memory(&self, st: &mut State, rec: &MemRec) {
        if self.inner.no_index {
            return;
        }
        let m = &rec.memory;
        let file = rec
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&rec.path)
            .trim_end_matches(".md")
            .to_string();
        let f = DocFields {
            title: &m.title,
            aliases: &m.aliases,
            file: &file,
            tags: &m.tags,
            summary: &m.summary,
            body: &m.body,
        };
        let State {
            index, analyzer, ..
        } = st;
        index.add(&m.id, &f, analyzer);
    }

    fn quote_broken(&self, m: &Memory) -> bool {
        let (Some(q), Some(n)) = (&m.origin.quote, &m.origin.note) else {
            return false;
        };
        if m.origin.hash.is_some() {
            return false;
        }
        match fs::read_to_string(self.inner.root.join(n)) {
            Ok(t) => !crate::analyzer::fold(&t).contains(&crate::analyzer::fold(q)),
            Err(_) => true,
        }
    }

    fn ingest_memory(
        &self,
        st: &mut State,
        rel: &str,
        text: &str,
        hash: String,
        sig: FileSig,
        read_at: i64,
    ) -> Result<bool> {
        let prior = st.path_ids.get(rel).cloned();
        if let Some(id) = &prior {
            if let Some(m) = st.mems.get_mut(id) {
                if m.hash == hash {
                    m.sig = sig;
                    m.read_at_ns = read_at;
                    return Ok(false);
                }
            }
        }
        match crate::model::parse_memory(text) {
            Ok(m) => {
                if let Some(other) = st.mems.get(&m.id) {
                    if other.path != rel && self.inner.root.join(&other.path).exists() {
                        st.invalid.insert(
                            rel.to_string(),
                            InvalidRec {
                                reason: "duplicate id".into(),
                                sig,
                            },
                        );
                        return Ok(false);
                    }
                    let old = other.path.clone();
                    st.path_ids.remove(&old);
                }
                if let Some(old) = &prior {
                    if old != &m.id {
                        self.drop_memory(st, old);
                    }
                }
                st.invalid.remove(rel);
                let quote_broken = self.quote_broken(&m);
                let rec = make_rec(m, rel, hash.clone(), sig, read_at, quote_broken);
                let id = rec.memory.id.clone();
                self.index_memory(st, &rec);
                st.path_ids.insert(rel.to_string(), id.clone());
                st.mems.insert(id.clone(), rec);
                self.note_external_memory(st, &id, Some(hash), rel);
                Ok(true)
            }
            Err(reason) => {
                if let Some(id) = prior {
                    self.drop_memory(st, &id);
                }
                st.invalid
                    .insert(rel.to_string(), InvalidRec { reason, sig });
                Ok(true)
            }
        }
    }

    fn note_external_memory(&self, st: &mut State, id: &str, hash: Option<String>, rel: &str) {
        if st.view.last_hash.get(id) == hash.as_ref() {
            return;
        }
        let mut e = self.event(EventKind::External, None);
        e.target = Some(id.to_string());
        e.path = Some(rel.to_string());
        e.kind = Some("memory".to_string());
        e.after_hash = hash;
        st.pending_external.push(e);
    }

    pub(crate) fn drop_memory(&self, st: &mut State, id: &str) {
        if let Some(rec) = st.mems.remove(id) {
            st.path_ids.remove(&rec.path);
            st.index.remove(id);
        }
    }

    fn forget_path(&self, st: &mut State, path: &str, external: bool) {
        if st.invalid.remove(path).is_some() {
            return;
        }
        if let Some(id) = st.path_ids.get(path).cloned() {
            self.drop_memory(st, &id);
            if external {
                self.note_external_memory(st, &id, None, path);
            }
            return;
        }
        if let Some(old) = st.notes.remove(path) {
            st.resolver_remove(&old);
            st.index.remove(&format!("note:{path}"));
            if external && st.view.last_hash.contains_key(path) {
                let mut e = self.event(EventKind::External, None);
                e.path = Some(path.to_string());
                e.kind = Some("deleted".to_string());
                st.pending_external.push(e);
            }
        }
    }

    /// Eventos `external` sob a trava, relendo o diário: dois processos não duplicam.
    fn flush_externals(&self, st: &mut State) {
        if st.pending_external.is_empty() {
            return;
        }
        let Some(_lock) = self.try_write_lock() else {
            return;
        };
        st.view.catch_up(&self.inner.root);
        let events = std::mem::take(&mut st.pending_external);
        for mut e in events {
            let key = e
                .target
                .clone()
                .or_else(|| e.path.clone())
                .unwrap_or_default();
            if st.view.last_hash.get(&key) == e.after_hash.as_ref() {
                continue;
            }
            // Evento adiado pela trava: se o arquivo já mudou de novo, o hash dele não vale mais.
            let current = match &e.target {
                Some(id) => st.mems.get(id).map(|r| r.hash.clone()),
                None => e
                    .path
                    .as_ref()
                    .and_then(|p| st.notes.get(p))
                    .map(|n| n.hash.clone()),
            };
            if current != e.after_hash {
                continue;
            }
            let _ = self.append(&mut e);
        }
        st.view.catch_up(&self.inner.root);
    }

    /// Reconcilia agora, sem prazo.
    pub fn reconcile(&self) -> Result<ReconcileReport> {
        let mut st = self.lock_state();
        self.refresh(&mut st, None, true)
    }

    /// Uma fatia de reconciliação. Solta o estado entre fatias, para consultas passarem na frente.
    pub fn reconcile_slice(&self, budget: Duration) -> Result<usize> {
        let mut st = self.lock_state();
        let force = !st.pending.is_empty() || !st.loaded;
        let _ = self.refresh(&mut st, Some(budget), force)?;
        Ok(st.pending.len())
    }

    // Instantâneo

    pub(crate) fn save_snapshot(&self, st: &mut State) -> Result<()> {
        if !st.loaded || !st.pending.is_empty() || self.inner.no_index {
            return Ok(());
        }
        // Refaz o livro de hashes a partir do diário, que é a fonte.
        let mut ledger = String::new();
        for (id, h) in &st.view.product_hash {
            ledger.push_str(id);
            ledger.push(' ');
            ledger.push_str(h);
            ledger.push('\n');
        }
        let tmp = util::tmp_path(&self.tmp_dir()?);
        if fs::write(&tmp, ledger).is_ok() {
            let _ = fs::rename(&tmp, self.private().join("ledger"));
        }
        // Por referência e com buffer no tamanho do anterior: gravar não pode inflar o processo.
        let meta = SnapshotOut {
            brain_id: &self.inner.config.id,
            notes: st.notes.values().collect(),
            mems: st.mems.values().collect(),
            invalid: &st.invalid,
            view: &st.view,
        };
        let hint = fs::metadata(self.private().join("snapshot.bin"))
            .map_or(1 << 20, |m| m.len() as usize + 4096);
        let data = encode_snapshot(&meta, &st.index, hint)?;
        let tmp = util::tmp_path(&self.tmp_dir()?);
        fs::write(&tmp, &data)?;
        // Sem fsync: é descartável.
        if let Err(e) = fs::rename(&tmp, self.private().join("snapshot.bin")) {
            let _ = fs::remove_file(&tmp);
            return Err(e.into());
        }
        store_projects(&self.inner.root, &st.projects());
        st.dirty = false;
        st.snapshot_saved = Some(Instant::now());
        Ok(())
    }


    /// Grava o instantâneo se houver mudança e já passou o intervalo, contando outros processos.
    pub fn maybe_snapshot(&self) -> Result<()> {
        let mut st = self.lock_state();
        if !st.dirty {
            return Ok(());
        }
        let recent = fs::metadata(self.private().join("snapshot.bin"))
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|d| d < SNAPSHOT_INTERVAL);
        if recent {
            return Ok(());
        }
        self.save_snapshot(&mut st)
    }

    /// Grava o instantâneo agora.
    pub fn flush(&self) -> Result<()> {
        let mut st = self.lock_state();
        self.refresh(&mut st, None, false)?;
        self.save_snapshot(&mut st)
    }

    // Notas

    fn fresh(&self) -> Result<MutexGuard<'_, State>> {
        let mut st = self.lock_state();
        self.refresh(&mut st, None, false)?;
        while !st.pending.is_empty() {
            self.refresh(&mut st, None, true)?;
        }
        Ok(st)
    }

    pub fn notes(&self) -> Result<Vec<NoteMeta>> {
        let st = self.fresh()?;
        let mut v: Vec<NoteMeta> = st.notes.values().map(State::note_meta).collect();
        v.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(v)
    }

    /// A pessoa abriu uma nota no app. Não muda nada: só alimenta as métricas de leitura.
    pub fn note_opened(&self, path: &str) {
        let mut e = self.event(EventKind::Tool, None);
        e.kind = Some("note_opened".into());
        e.path = Some(path.to_string());
        let _ = self.append(&mut e);
    }

    pub fn read_note(&self, path: &str) -> Result<NoteText> {
        let rel = paths::check_note_rel(path)?;
        let full = paths::safe_join(&self.inner.root, &rel)?;
        let bytes = fs::read(&full).map_err(|_| Error::NotFound(rel.clone()))?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let p = note::parse_note(&rel, &text);
        let meta = fs::symlink_metadata(&full)?;
        let sig = sig_of(&meta);
        let flags = note_flags(&rel, &p.title);
        Ok(NoteText {
            meta: NoteMeta {
                path: rel,
                title: p.title,
                aliases: p.aliases,
                tags: p.tags,
                links: p.links,
                hash: util::hash_bytes(&bytes),
                size: sig.size,
                modified: util::format_ts_seconds(sig.mtime_ns / 1000),
                flags,
            },
            text,
        })
    }

    fn note_event(
        &self,
        kind: &str,
        path: &str,
        before: Option<String>,
        after: Option<String>,
    ) -> Result<()> {
        let mut e = self.event(EventKind::Note, None);
        e.kind = Some(kind.to_string());
        e.path = Some(path.to_string());
        e.before_hash = before;
        e.after_hash = after;
        self.append(&mut e)
    }

    /// Compara e troca por hash. Sem `base_hash` só cria. Quem guarda o texto é o chamador.
    pub fn save_note(&self, path: &str, text: &str, base_hash: Option<&str>) -> Result<SaveResult> {
        let rel = paths::check_note_rel(path)?;
        let _lock = self.write_lock()?;
        let full = paths::safe_join(&self.inner.root, &rel)?;
        let current = fs::read(&full).ok();
        let cur_hash = current.as_deref().map(util::hash_bytes);
        if cur_hash.as_deref() != base_hash {
            return Ok(SaveResult::Conflict {
                current_hash: cur_hash.unwrap_or_default(),
                current_text: current
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
                    .unwrap_or_default(),
            });
        }
        let hash = util::hash_str(text);
        if cur_hash.as_deref() == Some(hash.as_str()) {
            return Ok(SaveResult::Saved { hash });
        }
        paths::create_parents(&self.inner.root, &full)?;
        let mode = file_mode(&full);
        util::atomic_write(&self.tmp_dir()?, &full, text.as_bytes(), mode)?;
        self.note_event("saved", &rel, cur_hash, Some(hash.clone()))?;
        let mut st = self.lock_state();
        self.after_note_change(&mut st, &rel, text, &hash)?;
        Ok(SaveResult::Saved { hash })
    }

    fn after_note_change(&self, st: &mut State, rel: &str, text: &str, hash: &str) -> Result<()> {
        st.view.last_hash.insert(rel.to_string(), hash.to_string());
        let full = self.inner.root.join(rel);
        let meta = fs::symlink_metadata(&full)?;
        self.ingest_note(st, rel, text, hash.to_string(), sig_of(&meta), now_ns())?;
        st.recompute_superseded(util::now_micros());
        st.dirty = true;
        self.bump_generation(st);
        Ok(())
    }

    /// Renomeia e atualiza só os wikilinks que apontam para a nota. Recusa se capturaria link de outra.
    pub fn rename_note(&self, from: &str, to: &str) -> Result<RenameReport> {
        let from = paths::check_note_rel(from)?;
        let to = paths::check_note_rel(to)?;
        let _lock = self.write_lock()?;
        let mut st = self.lock_state();
        self.refresh(&mut st, None, true)?;
        let full_from = paths::safe_join(&self.inner.root, &from)?;
        let full_to = paths::safe_join(&self.inner.root, &to)?;
        if !full_from.is_file() {
            return Err(Error::NotFound(from));
        }
        if from == to {
            return Ok(RenameReport {
                from,
                to,
                ..Default::default()
            });
        }
        // Em sistema sem distinção de caixa, trocar só a caixa é o mesmo arquivo.
        let same_file =
            crate::analyzer::fold(&from) == crate::analyzer::fold(&to) && full_to.exists();
        if full_to.exists() && !same_file {
            return Err(Error::AlreadyExists(to));
        }
        let before = st.resolver();
        // Resolução antes e depois, em cima do mesmo conjunto de notas.
        let mut new_title = None;
        let mut new_aliases = Vec::new();
        let renamed_rec = st.notes.get(&from).cloned();
        if let Some(n) = &renamed_rec {
            new_title = Some(n.title.clone());
            new_aliases = n.aliases.clone();
        }
        let old_stem = stem_of(&from);
        if new_title.as_deref() == Some(old_stem) {
            new_title = Some(stem_of(&to).to_string());
        }
        let after = {
            let title = new_title
                .clone()
                .unwrap_or_else(|| stem_of(&to).to_string());
            let mut entries: Vec<LinkEntry<'_>> = st
                .notes
                .values()
                .filter(|n| n.path != from)
                .map(|n| LinkEntry {
                    path: &n.path,
                    title: &n.title,
                    aliases: &n.aliases,
                })
                .collect();
            entries.push(LinkEntry {
                path: &to,
                title: &title,
                aliases: &new_aliases,
            });
            Resolver::build(entries.into_iter())
        };

        let mut captures = Vec::new();
        let mut ops: Vec<PlanOp> = Vec::new();
        let mut updated = Vec::new();
        let mut total = 0usize;
        let mut paths_sorted: Vec<String> = st.notes.keys().cloned().collect();
        paths_sorted.sort();
        for path in paths_sorted {
            let full = self.inner.root.join(&path);
            let Ok(bytes) = fs::read(&full) else { continue };
            let text = String::from_utf8_lossy(&bytes).into_owned();
            if !text.contains("[[") {
                continue;
            }
            let (_, body) = crate::frontmatter::split(&text);
            let body_off = text.len() - body.len();
            let spans = wikilinks(body);
            let mut new_text = String::with_capacity(text.len());
            let mut cursor = 0usize;
            let mut changed = 0usize;
            for s in &spans {
                let was = before.resolve(&s.target);
                let now = after.resolve(&s.target);
                if was.as_deref() == Some(from.as_str()) {
                    let replacement = link_text(&s.target, &to, &after);
                    if replacement != s.target {
                        new_text.push_str(&text[cursor..body_off + s.start]);
                        new_text.push_str(&replacement);
                        cursor = body_off + s.end;
                        changed += 1;
                    }
                } else if was.is_some() && was != now {
                    captures.push(path.clone());
                }
            }
            if changed > 0 {
                new_text.push_str(&text[cursor..]);
                let dest = if path == from {
                    to.clone()
                } else {
                    path.clone()
                };
                ops.push(PlanOp::Write {
                    path: if path == from {
                        from.clone()
                    } else {
                        path.clone()
                    },
                    expected: util::hash_bytes(&bytes),
                    text: new_text,
                });
                updated.push(dest);
                total += changed;
            }
        }
        if !captures.is_empty() {
            captures.sort();
            captures.dedup();
            return Err(Error::RenameCapturesLinks {
                target: to,
                notes: captures,
            });
        }
        ops.push(PlanOp::Rename {
            from: from.clone(),
            to: to.clone(),
        });
        let plan = Plan {
            id: util::new_ulid(),
            ops,
        };
        self.run_plan(&plan)?;
        let mut ev = self.event(EventKind::Note, None);
        ev.kind = Some("renamed".into());
        ev.path = Some(from.clone());
        ev.reason = Some(format!("renamed to {to}"));
        self.append(&mut ev)?;
        self.forget_path(&mut st, &from, false);
        st.missing.remove(&from);
        st.view.last_hash.remove(&from);
        // Reindexa o que mudou, sem deixar para virar `external`.
        for p in updated.iter().chain(std::iter::once(&to)) {
            let full = self.inner.root.join(p);
            let bytes = fs::read(&full)?;
            let hash = util::hash_bytes(&bytes);
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let mut e = self.event(EventKind::Note, None);
            e.kind = Some("saved".into());
            e.path = Some(p.clone());
            e.after_hash = Some(hash.clone());
            let _ = self.append(&mut e);
            self.after_note_change(&mut st, p, &text, &hash)?;
        }
        Ok(RenameReport {
            from,
            to,
            updated_notes: updated,
            links_rewritten: total,
        })
    }

    /// Move para a lixeira interna. `restore_note` traz de volta.
    pub fn delete_note(&self, path: &str) -> Result<TrashEntry> {
        let rel = paths::check_note_rel(path)?;
        let _lock = self.write_lock()?;
        let full = paths::safe_join(&self.inner.root, &rel)?;
        if !full.is_file() {
            return Err(Error::NotFound(rel));
        }
        let id = util::new_ulid();
        let dir = self.private().join("trash").join(&id);
        paths::create_parents(&self.inner.root, &dir.join("x"))?;
        let entry = TrashEntry {
            id,
            path: rel.clone(),
            deleted: util::now_ts_seconds(),
        };
        fs::write(
            dir.join("entry.json"),
            serde_json::to_vec(&entry).map_err(|e| Error::Invalid(e.to_string()))?,
        )?;
        let before = util::hash_bytes(&fs::read(&full)?);
        fs::rename(&full, dir.join("note.md"))?;
        self.note_event("deleted", &rel, Some(before), None)?;
        let mut st = self.lock_state();
        self.forget_path(&mut st, &rel, false);
        st.view.last_hash.remove(&rel);
        st.recompute_superseded(util::now_micros());
        st.dirty = true;
        self.bump_generation(&mut st);
        Ok(entry)
    }

    pub fn trash(&self) -> Result<Vec<TrashEntry>> {
        let mut v = Vec::new();
        if let Ok(rd) = fs::read_dir(self.private().join("trash")) {
            for e in rd.flatten() {
                if let Ok(b) = fs::read(e.path().join("entry.json")) {
                    if let Ok(t) = serde_json::from_slice::<TrashEntry>(&b) {
                        v.push(t);
                    }
                }
            }
        }
        v.sort_by(|a, b| b.deleted.cmp(&a.deleted).then(b.id.cmp(&a.id)));
        Ok(v)
    }

    pub fn restore_note(&self, trash_id: &str) -> Result<String> {
        if !util::is_ulid(trash_id) {
            return Err(Error::Invalid("bad trash id".into()));
        }
        let _lock = self.write_lock()?;
        let dir = self.private().join("trash").join(trash_id);
        let entry: TrashEntry = serde_json::from_slice(
            &fs::read(dir.join("entry.json")).map_err(|_| Error::NotFound(trash_id.into()))?,
        )
        .map_err(|e| Error::Invalid(e.to_string()))?;
        let rel = paths::check_note_rel(&entry.path)?;
        let full = paths::safe_join(&self.inner.root, &rel)?;
        if full.exists() {
            return Err(Error::AlreadyExists(rel));
        }
        paths::create_parents(&self.inner.root, &full)?;
        fs::rename(dir.join("note.md"), &full)?;
        let _ = fs::remove_dir_all(&dir);
        let bytes = fs::read(&full)?;
        let hash = util::hash_bytes(&bytes);
        self.note_event("restored", &rel, None, Some(hash.clone()))?;
        let mut st = self.lock_state();
        self.after_note_change(&mut st, &rel, &String::from_utf8_lossy(&bytes), &hash)?;
        Ok(rel)
    }

    // Planos multi-arquivo: idempotentes, retomados na próxima trava.

    pub(crate) fn run_plan(&self, plan: &Plan) -> Result<()> {
        let dir = self.private().join("plans");
        if !dir.exists() {
            let _ = fs::create_dir(&dir);
        }
        let path = dir.join(format!("{}.json", plan.id));
        let data = serde_json::to_vec(plan).map_err(|e| Error::Invalid(e.to_string()))?;
        util::atomic_write(&self.tmp_dir()?, &path, &data, None)?;
        self.execute_plan(plan)?;
        let _ = fs::remove_file(&path);
        Ok(())
    }

    fn execute_plan(&self, plan: &Plan) -> Result<()> {
        let tmp = self.tmp_dir()?;
        for op in &plan.ops {
            match op {
                PlanOp::Write {
                    path,
                    expected,
                    text,
                } => {
                    let full = paths::safe_join(&self.inner.root, path)?;
                    let cur = fs::read(&full).ok().map(|b| util::hash_bytes(&b));
                    if cur.as_deref() == Some(expected.as_str()) {
                        paths::create_parents(&self.inner.root, &full)?;
                        util::atomic_write(&tmp, &full, text.as_bytes(), file_mode(&full))?;
                    }
                }
                PlanOp::Rename { from, to } => {
                    let f = paths::safe_join(&self.inner.root, from)?;
                    let t = paths::safe_join(&self.inner.root, to)?;
                    if f.exists() {
                        paths::create_parents(&self.inner.root, &t)?;
                        fs::rename(&f, &t)?;
                    }
                }
                PlanOp::Remove { path, expected } => {
                    let full = paths::safe_join(&self.inner.root, path)?;
                    let cur = fs::read(&full).ok().map(|b| util::hash_bytes(&b));
                    if cur.as_deref() == Some(expected.as_str()) {
                        fs::remove_file(&full)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Sob a trava: plano achado aqui é de processo morto.
    fn resume_plans(&self) -> Result<()> {
        let dir = self.private().join("plans");
        let Ok(rd) = fs::read_dir(&dir) else {
            return Ok(());
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            match fs::read(&p)
                .ok()
                .and_then(|b| serde_json::from_slice::<Plan>(&b).ok())
            {
                Some(plan) => {
                    self.execute_plan(&plan)?;
                    let _ = fs::remove_file(&p);
                }
                None => {
                    let _ = fs::remove_file(&p);
                }
            }
        }
        Ok(())
    }

    // Diário

    /// Eventos novos desde `cursor`. É a fonte do "ao vivo" do app.
    pub fn journal_tail(&self, cursor: &Cursor) -> Result<JournalBatch> {
        self.check_alive()?;
        Ok(journal::tail(&self.inner.root, cursor, 10_000))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum PlanOp {
    Write {
        path: String,
        expected: String,
        text: String,
    },
    Rename {
        from: String,
        to: String,
    },
    Remove {
        path: String,
        expected: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Plan {
    pub id: String,
    pub ops: Vec<PlanOp>,
}

fn file_mode(full: &Path) -> Option<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(full)
            .ok()
            .map(|m| m.permissions().mode() & 0o777)
    }
    #[cfg(not(unix))]
    {
        let _ = full;
        None
    }
}

fn stem_of(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md")
        .or_else(|| name.strip_suffix(".markdown"))
        .unwrap_or(name)
}

/// Texto do link novo: nome curto se resolve para a nota renomeada; senão caminho inteiro.
fn link_text(old_target: &str, to: &str, after: &Resolver) -> String {
    let stem = stem_of(to);
    let full = to.strip_prefix("notes/").unwrap_or(to);
    let full = full.strip_suffix(".md").unwrap_or(full);
    let candidate = if old_target.contains('/') { full } else { stem };
    if after.resolve(candidate).as_deref() == Some(to) {
        return candidate.to_string();
    }
    if after.resolve(full).as_deref() == Some(to) {
        return full.to_string();
    }
    to.to_string()
}

/// Cópia descartável da lista de projetos, para quem não abre o cofre inteiro.
pub(crate) const PROJECTS_FILE: &str = "projects.json";

/// Grava a cópia da lista de projetos, só quando ela mudou. Falhar aqui não é erro: é um cache.
pub(crate) fn store_projects(root: &Path, list: &crate::projects::Projects) {
    let Ok(json) = serde_json::to_vec(list) else { return };
    let dir = root.join(".omnimind");
    let path = dir.join(PROJECTS_FILE);
    if fs::read(&path).is_ok_and(|old| old == json) {
        return;
    }
    let tmp = dir.join("tmp").join(format!("projects-{}", util::new_ulid()));
    if fs::create_dir_all(dir.join("tmp")).is_ok() && fs::write(&tmp, json).is_ok() && fs::rename(&tmp, &path).is_err() {
        let _ = fs::remove_file(&tmp);
    }
}

/// Projetos como o último processo que abriu o cofre os viu. Sem o arquivo, nenhum tem nota.
pub fn cached_projects(root: &Path) -> crate::projects::Projects {
    fs::read(root.join(".omnimind").join(PROJECTS_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn project_from_cwd(cwd: &str) -> String {
    util::slug(cwd.trim_end_matches('/').rsplit('/').next().unwrap_or(cwd))
}

fn encode_snapshot(meta: &SnapshotOut<'_>, index: &Index, hint: usize) -> Result<Vec<u8>> {
    const HEADER: usize = 8 + 4 + 4 + 8 + 8;
    let mut out = Vec::with_capacity(hint.max(HEADER + 8));
    out.extend_from_slice(SNAPSHOT_MAGIC);
    put_u32(&mut out, SNAPSHOT_FORMAT);
    put_u32(&mut out, ANALYZER_VERSION);
    out.extend_from_slice(&[0u8; 16]);
    put_u32(&mut out, 0);
    serde_json::to_writer(&mut out, meta).map_err(|e| Error::Invalid(e.to_string()))?;
    let json_len = (out.len() - HEADER - 4) as u32;
    out[HEADER..HEADER + 4].copy_from_slice(&json_len.to_le_bytes());
    index.encode(&mut out);
    let payload_len = (out.len() - HEADER) as u64;
    let sum = util::fast_checksum(&out[HEADER..]);
    out[16..24].copy_from_slice(&payload_len.to_le_bytes());
    out[24..32].copy_from_slice(&sum.to_le_bytes());
    Ok(out)
}

fn decode_snapshot(data: &[u8]) -> Option<(SnapshotMeta, Index)> {
    let mut r = Reader::new(data);
    if r.bytes(8)? != SNAPSHOT_MAGIC || r.u32()? != SNAPSHOT_FORMAT || r.u32()? != ANALYZER_VERSION
    {
        return None;
    }
    let len = u64::from_le_bytes(r.bytes(8)?.try_into().ok()?) as usize;
    let sum = u64::from_le_bytes(r.bytes(8)?.try_into().ok()?);
    let payload = r.bytes(len)?;
    if util::fast_checksum(payload) != sum || r.pos != data.len() {
        return None;
    }
    let mut p = Reader::new(payload);
    let jlen = p.u32()? as usize;
    let meta: SnapshotMeta = serde_json::from_slice(p.bytes(jlen)?).ok()?;
    let index = Index::decode(&mut p)?;
    if p.pos != payload.len() {
        return None;
    }
    Some((meta, index))
}
