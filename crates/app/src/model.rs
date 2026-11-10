//! Modelo de exibição: o que as telas precisam saber, já pronto para desenhar.
//! A thread de dados monta instantâneos; a interface nunca espera por disco.

use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MemKind {
    Decision = 1,
    Procedure = 2,
    Concept = 3,
    Episode = 4,
    Preference = 5,
}

impl MemKind {
    pub fn label(self) -> &'static str {
        match self {
            MemKind::Decision => "Decisão",
            MemKind::Procedure => "Procedimento",
            MemKind::Concept => "Conceito",
            MemKind::Episode => "Episódio",
            MemKind::Preference => "Preferência",
        }
    }
    pub const ALL: [MemKind; 5] = [MemKind::Decision, MemKind::Procedure, MemKind::Concept, MemKind::Episode, MemKind::Preference];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemState {
    Active,
    Stale,
    Expired,
    Superseded,
    Quarantined,
    Archived,
}

impl MemState {
    pub fn label(self) -> &'static str {
        match self {
            MemState::Active => "Ativa",
            MemState::Stale => "Desatualizada",
            MemState::Expired => "Vencida",
            MemState::Superseded => "Substituída",
            MemState::Quarantined => "Em revisão",
            MemState::Archived => "Arquivada",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authority {
    Person,
    Agent,
    External,
}

impl Authority {
    pub fn label(self) -> &'static str {
        match self {
            Authority::Person => "Você",
            Authority::Agent => "IA",
            Authority::External => "Externa",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Note {
    pub path: String,
    /// Dia da última alteração, já em texto de tela.
    pub modified: String,
    pub title: String,
    pub folder: String,
    pub bytes: u64,
    pub covered: bool,
    pub memories: u32,
    pub aliases: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Memory {
    pub id: String,
    /// Muda quando o texto muda: é o que amarra uma aprovação ao texto visto.
    pub hash: String,
    pub kind: MemKind,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub state: MemState,
    pub authority: Authority,
    pub origin: String,
    pub client: String,
    pub session: String,
    pub quote: String,
    pub note: Option<String>,
    pub created: String,
    pub tags: Vec<String>,
    pub uses: u32,
    /// Por que ficou retida, quando ficou.
    pub held: Option<String>,
    /// Projeto da memória, já pelo nome da nota. `None`: geral, vale em todo projeto.
    pub project: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Note,
    Memory(MemKind),
}

#[derive(Clone, Debug)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub title: String,
    /// Uso útil acumulado: engrossa o galho e aumenta o ponto.
    pub weight: f32,
    /// Grupo, como índice em `Graph::regions`.
    pub region: u32,
    /// Projeto, como índice em `Graph::projects`. `u32::MAX`: nenhum.
    pub project: u32,
    /// Nota que declara o projeto: o centro dele no grafo.
    pub hub: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Graph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<(u32, u32, f32)>,
    /// Grupos: a categoria acima do projeto.
    pub regions: Vec<String>,
    pub projects: Vec<String>,
}

/// Um projeto do cofre, com ou sem nota.
#[derive(Clone, Debug, Default)]
pub struct ProjectInfo {
    pub slug: String,
    pub title: String,
    pub group: String,
    /// Nota que o declara. Sem ela o projeto é provisório: nasceu do nome de uma pasta.
    pub note: Option<String>,
    pub memories: u32,
    /// Veio de uma pasta descartável: não é projeto, é o que sobrou sem dono.
    pub ephemeral: bool,
}

#[derive(Clone, Debug)]
pub struct Change {
    pub event: String,
    pub verb: String,
    pub target: String,
    pub title: String,
    pub kind: u8,
    pub reason: String,
    pub before: String,
    pub after: String,
    pub undone: bool,
    pub undoable: bool,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub session: String,
    pub client: String,
    pub project: String,
    /// `Hoje`, `Ontem` ou a data.
    pub day: String,
    pub time: String,
    pub reads: u32,
    pub changes: Vec<Change>,
}

#[derive(Clone, Debug)]
pub struct ProfileLine {
    pub text: String,
    pub memory: String,
    pub authority: Authority,
}

#[derive(Clone, Debug)]
pub struct ProfileSection {
    pub title: String,
    pub lines: Vec<ProfileLine>,
}

#[derive(Clone, Debug)]
pub struct AgendaItem {
    pub kind: String,
    pub text: String,
}

pub use omnimind_core::connect::LinkState;

/// Um projeto ligado a este cofre por um cliente.
#[derive(Clone, Debug)]
pub struct Link {
    pub project: String,
    pub name: String,
    pub client: String,
    pub state: LinkState,
}

#[derive(Clone, Debug, Default)]
pub struct Client {
    pub name: String,
    pub last_seen: String,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub name: String,
    pub path: String,
    pub notes: Vec<Note>,
    pub memories: Vec<Memory>,
    pub graph: Graph,
    pub sessions: Vec<Session>,
    pub profile: Vec<ProfileSection>,
    pub agenda: Vec<AgendaItem>,
    pub quarantined: u32,
    pub metrics: crate::stats::Metrics,
    pub clients: Vec<Client>,
    pub links: Vec<Link>,
    pub projects: Vec<ProjectInfo>,
    /// Clientes ligados em todos os projetos de uma vez, pela pasta pessoal.
    pub global_links: Vec<String>,
    pub generation: u64,
    /// Nota → notas que apontam para ela.
    pub backlinks: std::collections::HashMap<String, Vec<String>>,
}

/// O que a IA acabou de fazer, para a interface reagir ao vivo.
#[derive(Clone, Debug)]
pub enum Live {
    Read { node: String, client: String },
    Searched { query: String, client: String, found: u32 },
    Wrote { node: String, title: String, client: String },
    Undid { node: String },
    SessionEnded,
}

pub type Snapshot = Arc<Model>;
