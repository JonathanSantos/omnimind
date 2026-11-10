use std::fmt;

/// Erro do núcleo. `code()` é estável e em inglês; a mensagem nunca traz caminho absoluto.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    BrainNotFound(String),
    AlreadyExists(String),
    InvalidPath { path: String, reason: String },
    NotFound(String),
    Busy,
    Io(String),
    Config(String),
    ClientNotAllowed(String),
    Invalid(String),
    RenameCapturesLinks { target: String, notes: Vec<String> },
    UndoRefused { reason: String, chain: Vec<String> },
    ExportRefused(String),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Error::BrainNotFound(_) => "brain_not_found",
            Error::AlreadyExists(_) => "already_exists",
            Error::InvalidPath { .. } => "invalid_path",
            Error::NotFound(_) => "not_found",
            Error::Busy => "busy",
            Error::Io(_) => "io",
            Error::Config(_) => "config",
            Error::ClientNotAllowed(_) => "client_not_allowed",
            Error::Invalid(_) => "invalid",
            Error::RenameCapturesLinks { .. } => "rename_captures_links",
            Error::UndoRefused { .. } => "undo_refused",
            Error::ExportRefused(_) => "export_refused",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BrainNotFound(p) => write!(f, "brain not found: {p}"),
            Error::AlreadyExists(p) => write!(f, "already exists: {p}"),
            Error::InvalidPath { path, reason } => write!(f, "invalid path {path:?}: {reason}"),
            Error::NotFound(p) => write!(f, "not found: {p}"),
            Error::Busy => write!(f, "write lock busy, retry"),
            Error::Io(m) => write!(f, "io error: {m}"),
            Error::Config(m) => write!(f, "config error: {m}"),
            Error::ClientNotAllowed(c) => write!(f, "client not allowed: {c}"),
            Error::Invalid(m) => write!(f, "invalid: {m}"),
            Error::RenameCapturesLinks { target, notes } => {
                write!(
                    f,
                    "rename to {target} would capture links in: {}",
                    notes.join(", ")
                )
            }
            Error::UndoRefused { reason, chain } => {
                write!(f, "undo refused: {reason}")?;
                if !chain.is_empty() {
                    write!(f, " (undo chain available: {})", chain.join(", "))?;
                }
                Ok(())
            }
            Error::ExportRefused(m) => write!(f, "export refused: {m}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        // `Display` de io::Error não inclui o caminho, então não vaza a raiz do brain.
        Error::Io(e.kind().to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
