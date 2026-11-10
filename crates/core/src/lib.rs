//! Núcleo do Omnimind. Veja `docs/specs/mvp/contracts.md` para a API e `core.md` para as regras.

pub mod analyzer;
pub mod apply;
pub mod brain;
pub mod config;
pub mod connect;
pub mod error;
pub mod frontmatter;
pub mod import;
pub mod index;
pub mod inspect;
pub mod journal;
pub mod lifecycle;
pub mod model;
pub mod note;
pub mod paths;
pub mod projects;
pub mod search;
pub mod signals;
pub mod simhash;
pub mod tags;
pub mod util;

pub use apply::norm_title as apply_norm_title;
pub use apply::UndoReport;
pub use brain::{Brain, BrainOptions, FileSig, ReconcileReport};
pub use config::{normalize_client, BrainConfig};
pub use error::{Error, Result};
pub use import::ImportReport;
pub use journal::{Cursor, Event, EventKind, JournalBatch, SessionInfo};
pub use lifecycle::*;
pub use model::*;
pub use projects::{Place, Project, Projects};
pub use search::{
    Audience, Coverage, Hit, ReadRequest, ReadResponse, SearchQuery, SearchResponse, SearchStatus,
};
pub use signals::{classify, strip_pasted, SignalKind};
