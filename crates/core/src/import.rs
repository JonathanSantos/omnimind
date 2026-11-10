//! `import-markdown`: pasta Obsidian ou brain do EdgeMind. Lê a origem, nunca escreve nela.

use crate::brain::Brain;
use crate::error::{Error, Result};
use crate::frontmatter::{self, get_list, get_str};
use crate::model::*;
use crate::paths;
use crate::tags::{self, TagResult};
use crate::util;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImportReport {
    pub notes: usize,
    pub memories: usize,
    pub preferences: usize,
    pub skipped: usize,
    pub dropped_tags: Vec<String>,
    pub outcomes: std::collections::BTreeMap<String, usize>,
}

fn walk(dir: &Path, rel: &str, out: &mut Vec<String>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for e in entries {
        let Ok(name) = e.file_name().into_string() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let child = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let Ok(meta) = e.path().symlink_metadata() else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            walk(&e.path(), &child, out);
        } else if paths::is_note_name(&name) {
            out.push(child);
        }
    }
}

fn outcome_label(o: &Outcome) -> String {
    match o {
        Outcome::Written => "written".into(),
        Outcome::Merged { .. } => "merged".into(),
        Outcome::Superseded { .. } => "superseded".into(),
        Outcome::Quarantined { .. } => "quarantined".into(),
        Outcome::Rejected { reason, .. } => format!("rejected:{reason}"),
        Outcome::Unchanged => "unchanged".into(),
    }
}

impl Brain {
    /// Importa notas e memórias. Autoridade: `confirmed_by: human` vira `person`; o resto, `agent`.
    pub fn import_markdown(&self, source: &Path) -> Result<ImportReport> {
        if !source.is_dir() {
            return Err(Error::NotFound("source folder".into()));
        }
        let source = source.canonicalize()?;
        if self.root().starts_with(&source) {
            return Err(Error::Invalid("source contains this brain".into()));
        }
        let mut report = ImportReport::default();
        let is_edgemind = source.join("memory").is_dir() && source.join("schema").is_dir();
        let mut files = Vec::new();
        if is_edgemind {
            walk(&source.join("notes"), "notes", &mut files);
        } else {
            walk(&source, "", &mut files);
        }
        for rel in files {
            let dest = if rel.starts_with("notes/") {
                rel.clone()
            } else {
                format!("notes/{rel}")
            };
            let Ok(bytes) = fs::read(source.join(&rel)) else {
                report.skipped += 1;
                continue;
            };
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let cur = self.read_note(&dest).ok().map(|n| n.meta.hash);
            if cur.as_deref() == Some(util::hash_str(&text).as_str()) {
                continue;
            }
            if cur.is_some() {
                report.skipped += 1;
                continue;
            }
            match self.save_note(&dest, &text, None)? {
                SaveResult::Saved { .. } => report.notes += 1,
                SaveResult::Conflict { .. } => report.skipped += 1,
            }
        }
        if is_edgemind {
            self.import_edgemind_memories(&source, &mut report)?;
        }
        Ok(report)
    }

    fn import_edgemind_memories(&self, source: &Path, report: &mut ImportReport) -> Result<()> {
        let mut files = Vec::new();
        walk(&source.join("memory"), "memory", &mut files);
        files.retain(|f| f != "memory/index.md");
        // Substituídas primeiro, para `supersedes` encontrar o id.
        let mut parsed = Vec::new();
        for f in files {
            let Ok(text) = fs::read_to_string(source.join(&f)) else {
                report.skipped += 1;
                continue;
            };
            let (yaml, body) = frontmatter::split(&text);
            let Some(v) = yaml.and_then(frontmatter::parse_yaml) else {
                report.skipped += 1;
                continue;
            };
            parsed.push((f, v, body.to_string()));
        }
        parsed.sort_by_key(|(_, v, _)| !supersedes_of(v).is_empty());
        let actor = Actor::import();
        for (file, v, body) in parsed {
            let dir_type = file.split('/').nth(1).unwrap_or("");
            let mem_type = get_str(&v, "type")
                .and_then(|t| MemoryType::parse(&t))
                .or_else(|| MemoryType::parse(dir_type));
            let mut tag_list = Vec::new();
            for t in get_list(&v, "tags") {
                match tags::normalize(&t) {
                    TagResult::Ok(_) => tag_list.push(t),
                    _ => report.dropped_tags.push(t),
                }
            }
            let source_map = v.get("source");
            let note = source_map.and_then(|s| get_str(s, "note"));
            let confirmed = get_str(&v, "confirmed_by").as_deref() == Some("human");
            let note_hash = self.edgemind_stale_hash(note.as_deref(), source_map);
            let draft = MemoryDraft {
                mem_type,
                title: get_str(&v, "title").unwrap_or_default(),
                summary: get_str(&v, "summary").unwrap_or_default(),
                body,
                aliases: get_list(&v, "aliases"),
                tags: tag_list,
                origin_kind: Some(OriginKind::Import),
                note: note.filter(|n| self.root().join(n).is_file()),
                supersedes: supersedes_of(&v),
                import: Some(ImportMeta {
                    id: get_str(&v, "id"),
                    created: get_str(&v, "created_at"),
                    updated: get_str(&v, "updated_at"),
                    authority: Some(if confirmed {
                        Authority::Person
                    } else {
                        Authority::Agent
                    }),
                    client: get_str(&v, "author"),
                    session: source_map.and_then(|s| get_str(s, "session")),
                    archived: None,
                    note_hash,
                }),
                ..Default::default()
            };
            let a = self.apply(Change::Write(draft), actor.clone())?;
            *report
                .outcomes
                .entry(outcome_label(&a.outcome))
                .or_insert(0) += 1;
            if a.is_stored() {
                report.memories += 1;
            }
        }
        // Preferências da pessoa, uma por linha.
        if let Ok(text) = fs::read_to_string(source.join("schema").join("preferences.md")) {
            for line in text.lines() {
                let Some(item) = line.trim().strip_prefix("- ") else {
                    continue;
                };
                let (title, summary) = item.split_once(" — ").unwrap_or((item, item));
                let draft = MemoryDraft {
                    mem_type: Some(MemoryType::Preference),
                    title: title.trim().to_string(),
                    summary: summary.trim().to_string(),
                    origin_kind: Some(OriginKind::Import),
                    quote: Some(item.trim().to_string()),
                    import: Some(ImportMeta {
                        authority: Some(Authority::Person),
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                let a = self.apply(Change::Write(draft), actor.clone())?;
                *report
                    .outcomes
                    .entry(outcome_label(&a.outcome))
                    .or_insert(0) += 1;
                if a.is_stored() {
                    report.preferences += 1;
                }
            }
        }
        Ok(())
    }
}

impl Brain {
    /// No EdgeMind `source.hash` são 8 hex do corpo sem frontmatter. Se não bate, a memória já estava velha.
    fn edgemind_stale_hash(&self, note: Option<&str>, source: Option<&Value>) -> Option<String> {
        let want = source.and_then(|s| get_str(s, "hash"))?;
        let text = fs::read_to_string(self.root().join(note?)).ok()?;
        let (_, body) = frontmatter::split(&text);
        let cur = util::hash_str(&body.replace("\r\n", "\n"));
        (cur[..8] != want).then_some(want)
    }
}

fn supersedes_of(v: &Value) -> Vec<String> {
    let Some(Value::Array(edges)) = v.get("edges") else {
        return Vec::new();
    };
    edges
        .iter()
        .filter(|e| get_str(e, "type").as_deref() == Some("supersedes"))
        .filter_map(|e| get_str(e, "to"))
        .filter(|t| util::is_ulid(t))
        .collect()
}
