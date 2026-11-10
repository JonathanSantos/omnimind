//! Tipos do formato em disco e do contrato com o app.

use crate::error::Result;
use crate::frontmatter::{self, get_list, get_str, yaml_list, yaml_str};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Decision,
    Procedure,
    Concept,
    Episode,
    Preference,
}

impl MemoryType {
    pub const ALL: [MemoryType; 5] = [
        MemoryType::Decision,
        MemoryType::Procedure,
        MemoryType::Concept,
        MemoryType::Episode,
        MemoryType::Preference,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            MemoryType::Decision => "decision",
            MemoryType::Procedure => "procedure",
            MemoryType::Concept => "concept",
            MemoryType::Episode => "episode",
            MemoryType::Preference => "preference",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "decision" | "decisions" | "decisao" | "decisão" => Some(MemoryType::Decision),
            "procedure" | "procedures" | "procedimento" => Some(MemoryType::Procedure),
            "concept" | "concepts" | "conceito" => Some(MemoryType::Concept),
            "episode" | "episodes" | "episodio" | "episódio" => Some(MemoryType::Episode),
            "preference" | "preferences" | "preferencia" | "preferência" => {
                Some(MemoryType::Preference)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Authority {
    External,
    Agent,
    Person,
}

impl Authority {
    pub fn as_str(self) -> &'static str {
        match self {
            Authority::Person => "person",
            Authority::Agent => "agent",
            Authority::External => "external",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "person" => Some(Authority::Person),
            "agent" => Some(Authority::Agent),
            "external" => Some(Authority::External),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginKind {
    Correction,
    Request,
    Session,
    Note,
    Tool,
    Consolidation,
    Import,
}

impl OriginKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OriginKind::Correction => "correction",
            OriginKind::Request => "request",
            OriginKind::Session => "session",
            OriginKind::Note => "note",
            OriginKind::Tool => "tool",
            OriginKind::Consolidation => "consolidation",
            OriginKind::Import => "import",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "correction" => Some(OriginKind::Correction),
            "request" => Some(OriginKind::Request),
            "session" => Some(OriginKind::Session),
            "note" => Some(OriginKind::Note),
            "tool" | "web" => Some(OriginKind::Tool),
            "consolidation" => Some(OriginKind::Consolidation),
            "import" => Some(OriginKind::Import),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryState {
    Active,
    Stale,
    Expired,
    Superseded,
    Quarantined,
    Archived,
}

impl MemoryState {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryState::Active => "active",
            MemoryState::Stale => "stale",
            MemoryState::Expired => "expired",
            MemoryState::Superseded => "superseded",
            MemoryState::Quarantined => "quarantined",
            MemoryState::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Origin {
    pub kind: OriginKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    #[serde(rename = "type")]
    pub mem_type: MemoryType,
    pub title: String,
    pub summary: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub authority: Authority,
    pub origin: Origin,
    pub created: String,
    pub updated: String,
    pub supersedes: Vec<String>,
    pub expires: Option<String>,
    pub archived: Option<String>,
    /// Decisão do portão. Campo gravado porque nem todo campo inspecionado vai para o arquivo.
    pub quarantine: Option<String>,
    pub promoted: Option<String>,
    pub promoted_hash: Option<String>,
    pub body: String,
    /// Chaves desconhecidas do frontmatter, preservadas em texto.
    #[serde(default)]
    pub extra: String,
}

const KNOWN_KEYS: &[&str] = &[
    "id",
    "type",
    "title",
    "summary",
    "aliases",
    "tags",
    "authority",
    "origin",
    "created",
    "updated",
    "supersedes",
    "expires",
    "archived",
    "quarantine",
    "promoted",
    "promoted_hash",
];

pub fn parse_memory(text: &str) -> std::result::Result<Memory, String> {
    let (yaml, body) = frontmatter::split(text);
    let yaml = yaml.ok_or("missing frontmatter")?;
    let v = frontmatter::parse_yaml(yaml).ok_or("invalid frontmatter")?;
    let id = get_str(&v, "id").ok_or("missing id")?;
    if !crate::util::is_ulid(&id) {
        return Err("invalid id".into());
    }
    let mem_type = get_str(&v, "type")
        .and_then(|t| MemoryType::parse(&t))
        .ok_or("invalid type")?;
    let title = get_str(&v, "title").unwrap_or_default();
    if title.trim().is_empty() {
        return Err("missing title".into());
    }
    let o = v.get("origin");
    let ostr = |k: &str| o.and_then(|x| get_str(x, k));
    let origin = Origin {
        kind: ostr("kind")
            .and_then(|k| OriginKind::parse(&k))
            .unwrap_or(OriginKind::Session),
        client: ostr("client"),
        session: ostr("session"),
        quote: ostr("quote"),
        note: ostr("note"),
        hash: ostr("hash"),
    };
    Ok(Memory {
        id,
        mem_type,
        title,
        summary: get_str(&v, "summary").unwrap_or_default(),
        aliases: get_list(&v, "aliases"),
        tags: get_list(&v, "tags"),
        authority: get_str(&v, "authority")
            .and_then(|a| Authority::parse(&a))
            .unwrap_or(Authority::External),
        origin,
        created: get_str(&v, "created").unwrap_or_default(),
        updated: get_str(&v, "updated").unwrap_or_default(),
        supersedes: get_list(&v, "supersedes"),
        expires: get_str(&v, "expires"),
        archived: get_str(&v, "archived"),
        quarantine: get_str(&v, "quarantine"),
        promoted: get_str(&v, "promoted"),
        promoted_hash: get_str(&v, "promoted_hash"),
        body: body.trim_start_matches('\n').to_string(),
        extra: frontmatter::unknown_blocks(yaml, KNOWN_KEYS),
    })
}

pub fn render_memory(m: &Memory) -> String {
    let mut s = String::with_capacity(m.body.len() + 512);
    s.push_str("---\n");
    let mut kv = |k: &str, v: String| {
        s.push_str(k);
        s.push_str(": ");
        s.push_str(&v);
        s.push('\n');
    };
    kv("id", m.id.clone());
    kv("type", m.mem_type.as_str().to_string());
    kv("title", yaml_str(&m.title));
    kv("summary", yaml_str(&m.summary));
    kv("aliases", yaml_list(&m.aliases));
    kv("tags", yaml_list(&m.tags));
    kv("authority", m.authority.as_str().to_string());
    s.push_str("origin:\n");
    s.push_str(&format!("  kind: {}\n", m.origin.kind.as_str()));
    let o = &m.origin;
    for (k, v) in [
        ("client", &o.client),
        ("session", &o.session),
        ("quote", &o.quote),
        ("note", &o.note),
        ("hash", &o.hash),
    ] {
        if let Some(v) = v {
            s.push_str(&format!("  {k}: {}\n", yaml_str(v)));
        }
    }
    let mut kv = |k: &str, v: String| {
        s.push_str(k);
        s.push_str(": ");
        s.push_str(&v);
        s.push('\n');
    };
    kv("created", yaml_str(&m.created));
    kv("updated", yaml_str(&m.updated));
    if !m.supersedes.is_empty() {
        kv("supersedes", yaml_list(&m.supersedes));
    }
    if let Some(e) = &m.expires {
        kv("expires", yaml_str(e));
    }
    if let Some(e) = &m.archived {
        kv("archived", yaml_str(e));
    }
    if let Some(e) = &m.quarantine {
        kv("quarantine", yaml_str(e));
    }
    if let Some(e) = &m.promoted {
        kv("promoted", yaml_str(e));
    }
    if let Some(e) = &m.promoted_hash {
        kv("promoted_hash", yaml_str(e));
    }
    s.push_str(&m.extra);
    s.push_str("---\n");
    s.push_str(&m.body);
    if !m.body.is_empty() && !m.body.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// Metadado leve para listas no app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryMeta {
    pub id: String,
    #[serde(rename = "type")]
    pub mem_type: MemoryType,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub authority: Authority,
    pub state: MemoryState,
    pub origin: Origin,
    pub created: String,
    pub updated: String,
    pub path: String,
    pub hash: String,
    pub supersedes: Vec<String>,
    pub superseded_by: Option<String>,
    /// Código estável em inglês; a interface traduz. Lista em `contracts.md`.
    pub quarantine_reason: Option<String>,
    /// `unverified_authority` quando o arquivo declara `person` sem evento no diário.
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryDetail {
    pub meta: MemoryMeta,
    pub memory: Memory,
    /// Texto cru do arquivo. Pode ter invisíveis; a interface deve usar `revealed`.
    pub text: String,
    pub revealed: String,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteMeta {
    pub path: String,
    pub title: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub links: Vec<String>,
    pub hash: String,
    pub size: u64,
    pub modified: String,
    /// `invisible_name` e `instruction_like`, sobre título e nome do arquivo.
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoteText {
    pub meta: NoteMeta,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SaveResult {
    Saved {
        hash: String,
    },
    Conflict {
        current_hash: String,
        current_text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RenameReport {
    pub from: String,
    pub to: String,
    pub updated_notes: Vec<String>,
    pub links_rewritten: usize,
}

/// Resultado de `delete_note`: a nota vai para a lixeira interna e pode voltar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrashEntry {
    pub id: String,
    pub path: String,
    pub deleted: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Person,
    Agent,
    Hook,
    System,
    External,
    Import,
}

/// Quem pede a mudança. Vem do transporte, nunca de campo livre da requisição.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Actor {
    pub kind: ActorKind,
    pub client: String,
    pub session: Option<String>,
    pub cwd: Option<String>,
}

impl Actor {
    pub fn person() -> Self {
        Actor {
            kind: ActorKind::Person,
            client: "person".into(),
            session: None,
            cwd: None,
        }
    }
    pub fn agent(client: &str, session: &str) -> Self {
        Actor {
            kind: ActorKind::Agent,
            client: client.into(),
            session: Some(session.into()),
            cwd: None,
        }
    }
    pub fn system() -> Self {
        Actor {
            kind: ActorKind::System,
            client: "omnimind".into(),
            session: None,
            cwd: None,
        }
    }
    pub fn external(client: &str) -> Self {
        Actor {
            kind: ActorKind::External,
            client: client.into(),
            session: None,
            cwd: None,
        }
    }
    pub fn import() -> Self {
        Actor {
            kind: ActorKind::Import,
            client: "import".into(),
            session: None,
            cwd: None,
        }
    }
    pub fn with_cwd(mut self, cwd: &str) -> Self {
        self.cwd = Some(cwd.to_string());
        self
    }
    pub fn with_session(mut self, s: &str) -> Self {
        self.session = Some(s.to_string());
        self
    }
}

/// Rascunho de memória. Sem `id`, cria; com `id`, altera e exige `base_hash`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MemoryDraft {
    pub key: Option<String>,
    pub id: Option<String>,
    pub base_hash: Option<String>,
    pub mem_type: Option<MemoryType>,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub origin_kind: Option<OriginKind>,
    pub quote: Option<String>,
    pub note: Option<String>,
    pub supersedes: Vec<String>,
    pub expires: Option<String>,
    pub reason: Option<String>,
    /// Só `ActorKind::Import` usa: preserva id, datas e autoridade do material importado.
    pub import: Option<ImportMeta>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportMeta {
    pub id: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub authority: Option<Authority>,
    pub client: Option<String>,
    pub session: Option<String>,
    pub archived: Option<String>,
    /// Hash de origem preservado quando a nota mudou depois da memória: a importação mantém `stale`.
    pub note_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum Change {
    Write(MemoryDraft),
    Archive {
        id: String,
        base_hash: Option<String>,
        reason: Option<String>,
    },
    Restore {
        id: String,
        base_hash: Option<String>,
        reason: Option<String>,
    },
    /// Tira da quarentena. Só `Actor::person`.
    Promote {
        id: String,
        base_hash: Option<String>,
        reason: Option<String>,
    },
    /// Linha do retrato editada na interface: vira correção de autoridade `person`.
    CorrectLine {
        id: String,
        base_hash: Option<String>,
        text: String,
    },
    /// Funde `from` em `into`, somando aliases e marcando `supersedes`.
    Merge {
        into: String,
        from: String,
        base_hash: Option<String>,
        reason: Option<String>,
    },
    /// Acrescenta alias aprendido, sem mudar autoridade.
    AddAlias {
        id: String,
        alias: String,
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    Written,
    Merged { into: String },
    Superseded { ids: Vec<String> },
    Quarantined { reason: String },
    Rejected { reason: String, hint: String },
    Unchanged,
}

/// Vizinho devolvido por uma escrita.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Neighbor {
    pub id: String,
    pub title: String,
    pub state: MemoryState,
    pub relation: String,
}

/// Retorno de `Brain::apply`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Applied {
    pub outcome: Outcome,
    pub id: Option<String>,
    pub path: Option<String>,
    pub rule: String,
    pub hash: Option<String>,
    pub event: Option<String>,
    pub warnings: Vec<String>,
    pub neighbors: Vec<Neighbor>,
    pub tags: Vec<String>,
    pub tag_vocabulary: Vec<String>,
    pub current_text: Option<String>,
}

impl Applied {
    pub fn is_stored(&self) -> bool {
        matches!(
            self.outcome,
            Outcome::Written
                | Outcome::Merged { .. }
                | Outcome::Superseded { .. }
                | Outcome::Unchanged
        )
    }
}

pub type ApplyResult = Result<Applied>;

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Memory {
        Memory {
            id: crate::util::new_ulid(),
            mem_type: MemoryType::Preference,
            title: "Prefere: commits pequenos".into(),
            summary: "Uma frase".into(),
            aliases: vec!["commits atômicos".into(), "small: commits".into()],
            tags: vec!["project/rota-certa".into()],
            authority: Authority::Person,
            origin: Origin {
                kind: OriginKind::Correction,
                client: Some("claude-code".into()),
                session: Some("abc".into()),
                quote: Some("não, faz commits menores".into()),
                note: Some("notes/x.md".into()),
                hash: Some("7f3a".into()),
            },
            created: "2026-10-31T19:20:00Z".into(),
            updated: "2026-10-31T19:20:00Z".into(),
            supersedes: vec![crate::util::new_ulid()],
            expires: Some("2026-12-31".into()),
            archived: None,
            quarantine: None,
            promoted: None,
            promoted_hash: None,
            body: "Corpo\n---\nnao e cabecalho\n".into(),
            extra: "custom:\n  a: 1\n".into(),
        }
    }

    #[test]
    fn memory_round_trip() {
        let m = sample();
        let text = render_memory(&m);
        let back = parse_memory(&text).unwrap();
        assert_eq!(back, m);
        assert_eq!(render_memory(&back), text);
    }

    #[test]
    fn hostile_files_never_panic() {
        for t in [
            "",
            "---",
            "---\n---\n",
            "---\nid: ../../evil\ntype: decision\ntitle: x\n---\n",
            "---\nid: 01M69V237AYAMGTWMHQX2A20YR\ntype: nope\ntitle: x\n---\n",
            "---\nid: 01M69V237AYAMGTWMHQX2A20YR\ntype: decision\ntitle: [1\n",
        ] {
            assert!(parse_memory(t).is_err(), "{t}");
        }
    }

    #[test]
    fn missing_authority_is_external() {
        let t = "---\nid: 01M69V237AYAMGTWMHQX2A20YR\ntype: decision\ntitle: x\n---\ncorpo\n";
        assert_eq!(parse_memory(t).unwrap().authority, Authority::External);
    }
}
