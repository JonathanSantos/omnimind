//! Conversa entre a interface e a thread de dados. Nada aqui bloqueia o quadro.
use crate::model::{Live, Snapshot};

#[derive(Clone, Debug)]
pub enum Cmd {
    Refresh,
    OpenNote(String),
    SaveNote { path: String, text: String, base: String },
    /// Última gravação de uma nota que está saindo da tela: em conflito, vira nota irmã.
    SaveLeaving { path: String, text: String, base: String },
    KeepBoth { path: String, text: String },
    CreateNote(String),
    DailyNote,
    RenameNote { path: String, title: String },
    DeleteNote(String),
    LoadMemory(String),
    Undo(String),
    UndoSession(String),
    Forget(String),
    /// `hash` é o do texto que a pessoa viu: se mudou, a aprovação não vale.
    Promote { id: String, hash: String },
    Correct { memory: Option<String>, text: String },
    Search { query: String, seq: u64 },
    Connect { client: String, project: String },
    Disconnect { client: String, project: String },
    ForgetLink { client: String, project: String },
    OpenBrain(String),
    CreateBrain(String),
    Sleep,
    /// A partir daqui ninguém mais responde a conflito: o texto vai para uma nota irmã.
    Closing,
    Shutdown,
}

#[derive(Clone, Debug)]
pub struct Hit {
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub kind: u8,
    pub is_note: bool,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Snapshot(Snapshot),
    Note { path: String, text: String, hash: String },
    Saved { path: String, hash: String },
    Conflict { path: String, disk: String, hash: String },
    SaveFailed { path: String, reason: String },
    External { path: String, text: String, hash: String },
    Live(Live),
    Results { seq: u64, hits: Vec<Hit>, note: String },
    MemoryBody { id: String, hash: String, body: String },
    Toast(String),
    NoBrain,
}
