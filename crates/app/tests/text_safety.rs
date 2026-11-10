//! Regra central: nada digitado some sem escolha explícita.
//! Gerador aleatório sobre as transições, com o núcleo real por baixo. Cada semente é um brain novo.

use app::command::{Cmd, Msg};
use app::data;
use app::input::{Event, Key, Mods};
use app::views::notes::{NotesView, SaveState};
use app::Clipboard;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

struct Rng(u64);
impl Rng {
    fn n(&mut self, m: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % m as u64) as usize
    }
}

struct Rig {
    root: PathBuf,
    view: NotesView,
    tx: Sender<Cmd>,
    rx: Receiver<Msg>,
    join: Option<std::thread::JoinHandle<()>>,
    clip: Clipboard,
}

impl Rig {
    fn new(seed: u64) -> Self {
        let root = std::env::temp_dir().join(format!("omnimind-safety-{}-{seed}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let b = omnimind_core::Brain::init(&root, "Teste").expect("init");
        for n in ["a", "b", "c"] {
            b.save_note(&format!("notes/{n}.md"), &format!("# {n}\n\n"), None).expect("seed");
        }
        drop(b);
        let (tx, crx) = channel();
        let (mtx, rx) = channel();
        let join = data::spawn(crx, mtx, Box::new(|| {}), Some(root.to_string_lossy().into_owned()), false);
        Self { root, view: NotesView::new(), tx, rx, join: Some(join), clip: Clipboard::local() }
    }

    fn pump(&mut self, wait: Duration) {
        let end = Instant::now() + wait;
        loop {
            while let Ok(m) = self.rx.try_recv() {
                match m {
                    Msg::Note { path, text, hash } => self.view.opened(path, text, hash, &self.tx),
                    Msg::Saved { path, hash } => self.view.saved(&path, hash, &self.tx),
                    Msg::Conflict { path, disk, hash } => self.view.conflict(&path, disk, hash),
                    Msg::External { path, text, hash } => self.view.external(&path, text, hash),
                    Msg::SaveFailed { path, .. } => self.view.save_failed(&path),
                    _ => {}
                }
            }
            if Instant::now() >= end {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn settle(&mut self) {
        for _ in 0..200 {
            self.pump(Duration::from_millis(2));
            if self.view.save != SaveState::Saving {
                return;
            }
        }
    }

    fn open(&mut self, p: &str) {
        self.view.request_open(p, &self.tx);
        for _ in 0..400 {
            self.pump(Duration::from_millis(1));
            if self.view.path.as_deref() == Some(p) {
                return;
            }
        }
    }

    fn close(mut self) -> PathBuf {
        self.view.flush_for_exit(&self.tx);
        let _ = self.tx.send(Cmd::Shutdown);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
        self.root.clone()
    }
}

fn walk(d: &std::path::Path, out: &mut String) {
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        if e.path().is_dir() {
            walk(&e.path(), out);
        } else if let Ok(t) = std::fs::read_to_string(e.path()) {
            out.push_str(&t);
        }
    }
}

fn run(seed: u64) {
    let mut r = Rig::new(seed);
    let mut g = Rng(seed.wrapping_mul(2654435761) | 1);
    r.pump(Duration::from_millis(30));
    r.open("notes/a.md");
    let mut typed: HashSet<String> = HashSet::new();
    let mut tok = 0;
    for _ in 0..60 {
        let cur = r.view.path.clone().expect("open note");
        match g.n(10) {
            0..=4 => {
                // Cada digitação carrega um marcador único: dá para provar que chegou ao disco.
                tok += 1;
                let t = format!("tk{seed}x{tok}ç ");
                typed.insert(t.trim().to_string());
                r.view.event(&Event::Key(Key::Down, Mods { cmd: true, ..Default::default() }), &mut r.clip);
                r.view.event(&Event::Text(t), &mut r.clip);
            }
            5 | 6 => {
                let p = ["notes/a.md", "notes/b.md", "notes/c.md"][g.n(3)];
                if !matches!(r.view.save, SaveState::Conflict { .. }) {
                    r.open(p);
                }
            }
            7 => {
                r.view.flush(&r.tx);
                r.settle();
            }
            8 => {
                // Outro processo grava no disco, pela escrita atômica de um editor qualquer.
                r.pump(Duration::from_millis(1));
                let f = r.root.join(&cur);
                let old = std::fs::read_to_string(&f).unwrap_or_default();
                let tmp = f.with_extension("tmp");
                std::fs::write(&tmp, format!("{old}\nexterno {seed}\n")).expect("write");
                std::fs::rename(&tmp, &f).expect("rename");
                r.pump(Duration::from_millis(280));
            }
            _ => {
                if matches!(r.view.save, SaveState::Conflict { .. }) {
                    let choice = g.n(2) as u8;
                    r.view.resolve(choice, &r.tx);
                    r.settle();
                }
            }
        }
    }
    if matches!(r.view.save, SaveState::Conflict { .. }) && g.n(2) == 0 {
        r.view.resolve(1, &r.tx);
    }
    let root = r.close();
    let mut text = String::new();
    walk(&root.join("notes"), &mut text);
    for t in &typed {
        assert!(text.contains(t), "seed {seed}: token {t} lost");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn random_transitions_never_lose_typed_text() {
    let n: u64 = std::env::var("SAFETY_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(40);
    for seed in 1..=n {
        run(seed);
    }
}

#[test]
fn control_positive_detects_a_lost_token() {
    // Sem isto, o teste acima poderia estar passando sem olhar nada.
    let mut r = Rig::new(9_999);
    r.pump(Duration::from_millis(30));
    r.open("notes/a.md");
    r.view.event(&Event::Text("presente ".into()), &mut r.clip);
    let root = r.close();
    let mut text = String::new();
    walk(&root.join("notes"), &mut text);
    assert!(text.contains("presente"));
    assert!(!text.contains("ausente"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn quit_during_conflict_keeps_both_versions() {
    let mut r = Rig::new(7_777);
    r.pump(Duration::from_millis(30));
    r.open("notes/a.md");
    r.view.event(&Event::Text("minha-frase ".into()), &mut r.clip);
    let f = r.root.join("notes/a.md");
    std::fs::write(&f, "versão de fora\n").expect("write");
    r.pump(Duration::from_millis(350));
    assert!(matches!(r.view.save, SaveState::Conflict { .. }), "{:?}", r.view.save);
    let root = r.close();
    let mut text = String::new();
    walk(&root.join("notes"), &mut text);
    assert!(text.contains("minha-frase"), "my text lost");
    assert!(text.contains("versão de fora"), "external text lost");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn close_with_pending_typing_writes_the_last_keystroke() {
    let mut r = Rig::new(5_555);
    r.pump(Duration::from_millis(30));
    r.open("notes/b.md");
    r.view.event(&Event::Text("última tecla".into()), &mut r.clip);
    let root = r.close();
    let t = std::fs::read_to_string(root.join("notes/b.md")).expect("read");
    assert!(t.contains("última tecla"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_new_note_never_replaces_unsaved_text() {
    let mut r = Rig::new(77);
    r.pump(Duration::from_millis(30));
    r.open("notes/a.md");
    let mut clip = Clipboard::local();
    r.view.event(&Event::Text("TOKEN-PENDENTE".into()), &mut clip);
    // ⌘D logo depois de digitar: a nota nova chega sem passar por `request_open`.
    let _ = r.tx.send(Cmd::CreateNote("Outra".into()));
    for _ in 0..200 {
        r.pump(Duration::from_millis(2));
        if r.view.path.as_deref() != Some("notes/a.md") {
            break;
        }
    }
    assert_ne!(r.view.path.as_deref(), Some("notes/a.md"));
    let root = r.close();
    let mut all = String::new();
    walk(&root.join("notes"), &mut all);
    assert!(all.contains("TOKEN-PENDENTE"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_new_note_during_a_conflict_keeps_both_versions() {
    let mut r = Rig::new(78);
    r.pump(Duration::from_millis(30));
    r.open("notes/a.md");
    let mut clip = Clipboard::local();
    r.view.event(&Event::Text("TOKEN-CONFLITO".into()), &mut clip);
    r.view.conflict("notes/a.md", "disco".into(), "h2".into());
    let _ = r.tx.send(Cmd::CreateNote("Mais uma".into()));
    for _ in 0..200 {
        r.pump(Duration::from_millis(2));
        if r.view.path.as_deref() != Some("notes/a.md") {
            break;
        }
    }
    let root = r.close();
    let mut all = String::new();
    walk(&root.join("notes"), &mut all);
    assert!(all.contains("TOKEN-CONFLITO"));
    let _ = std::fs::remove_dir_all(root);
}
