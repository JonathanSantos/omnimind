#![allow(dead_code)]

use std::path::{Path, PathBuf};
use omnimind_core::{util, Actor, Applied, Brain, Change, MemoryDraft, MemoryType};

pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!("vz-{tag}-{}", util::new_ulid()));
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p.canonicalize().unwrap())
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn join(&self, s: &str) -> PathBuf {
        self.0.join(s)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn brain(tag: &str) -> (TempDir, Brain) {
    let d = TempDir::new(tag);
    let b = Brain::init(d.join("b"), "Teste").unwrap();
    (d, b)
}

pub fn agent(b: &Brain) -> Actor {
    Actor::agent("claude-code", b.session())
}

pub fn draft(title: &str, summary: &str) -> MemoryDraft {
    MemoryDraft {
        mem_type: Some(MemoryType::Concept),
        title: title.into(),
        summary: summary.into(),
        ..Default::default()
    }
}

pub fn write(b: &Brain, d: MemoryDraft) -> Applied {
    b.apply(Change::Write(d), agent(b)).unwrap()
}

/// Gerador xorshift com semente fixa: reprodutível, sem dependência.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0.max(1);
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

pub fn hash_tree(root: &Path) -> Vec<(String, String)> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n == ".omnimind" {
                continue;
            }
            let p = e.path();
            if p.is_dir() {
                walk(&p, base, out);
            } else if n.ends_with(".md") {
                out.push((
                    p.strip_prefix(base).unwrap().display().to_string(),
                    util::hash_bytes(&std::fs::read(&p).unwrap()),
                ));
            }
        }
    }
    let mut v = Vec::new();
    walk(root, root, &mut v);
    v.sort();
    v
}
