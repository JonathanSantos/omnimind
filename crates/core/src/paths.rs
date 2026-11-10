//! Tudo que vira caminho passa por aqui. Sem `..`, absoluto, oculto, nem symlink.

use crate::error::{Error, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

fn bad(path: &str, reason: &str) -> Error {
    Error::InvalidPath {
        path: path.chars().take(200).collect(),
        reason: reason.to_string(),
    }
}

/// Valida caminho relativo ao brain, com `/`. Não toca no disco.
pub fn check_rel(rel: &str) -> Result<String> {
    if rel.is_empty() || rel.len() > 1024 {
        return Err(bad(rel, "empty or too long"));
    }
    if rel.contains('\0') || rel.contains('\\') {
        return Err(bad(rel, "forbidden character"));
    }
    let lower = rel.to_ascii_lowercase();
    if lower.contains("%2e") || lower.contains("%2f") {
        return Err(bad(rel, "encoded traversal"));
    }
    let p = Path::new(rel);
    if p.is_absolute() {
        return Err(bad(rel, "absolute path"));
    }
    let mut parts = Vec::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => {
                let s = s.to_str().ok_or_else(|| bad(rel, "not utf-8"))?;
                if s.starts_with('.') {
                    return Err(bad(rel, "hidden component"));
                }
                parts.push(s);
            }
            Component::CurDir => {}
            _ => return Err(bad(rel, "traversal")),
        }
    }
    if parts.is_empty() {
        return Err(bad(rel, "empty"));
    }
    Ok(parts.join("/"))
}

/// Caminho de nota: validado, `.md`, fora de `memories/`.
pub fn check_note_rel(rel: &str) -> Result<String> {
    let r = check_rel(rel)?;
    if !is_note_name(&r) {
        return Err(bad(rel, "not a markdown file"));
    }
    if r == "memories" || r.starts_with("memories/") {
        return Err(bad(rel, "inside memories/"));
    }
    Ok(r)
}

pub fn is_note_name(p: &str) -> bool {
    let l = p.to_ascii_lowercase();
    l.ends_with(".md") || l.ends_with(".markdown")
}

/// Junta e garante que nenhum componente existente é symlink.
pub fn safe_join(root: &Path, rel: &str) -> Result<PathBuf> {
    let clean = check_rel(rel)?;
    let mut cur = root.to_path_buf();
    let mut exists = true;
    for part in clean.split('/') {
        cur.push(part);
        if exists {
            match fs::symlink_metadata(&cur) {
                Ok(m) if m.file_type().is_symlink() => return Err(bad(rel, "symlink refused")),
                Ok(_) => {}
                Err(_) => exists = false,
            }
        }
    }
    Ok(cur)
}

/// Cria diretórios um a um, só abaixo de `root`, que precisa existir.
pub fn create_parents(root: &Path, full: &Path) -> Result<()> {
    let Some(parent) = full.parent() else {
        return Ok(());
    };
    if !root.exists() {
        return Err(Error::BrainNotFound("root vanished".into()));
    }
    let rel = parent
        .strip_prefix(root)
        .map_err(|_| Error::Invalid("outside brain".into()))?;
    let mut cur = root.to_path_buf();
    for c in rel.components() {
        cur.push(c);
        match fs::symlink_metadata(&cur) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(Error::InvalidPath {
                    path: rel.display().to_string(),
                    reason: "symlink refused".into(),
                })
            }
            Ok(_) => {}
            Err(_) => match fs::create_dir(&cur) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e.into()),
            },
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traversal_refused() {
        for p in [
            "../x.md",
            "a/../../x.md",
            "/etc/passwd",
            "a\\b.md",
            ".git/config",
            "a/.hidden/b.md",
            "daily/..%2fx.md",
            "",
            "a\0b",
        ] {
            assert!(check_rel(p).is_err(), "{p}");
        }
        assert_eq!(check_rel("./a//b.md").unwrap(), "a/b.md");
        assert!(check_note_rel("memories/x.md").is_err());
        assert!(check_note_rel("a.txt").is_err());
    }
}
