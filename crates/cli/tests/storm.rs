//! Tempestade multi-processo: binários reais, sem perda nem pânico.

mod common;

use common::*;
use serde_json::json;
use std::collections::BTreeSet;
use std::thread;

const WRITERS: usize = 8;
const READERS: usize = 8;
const WRITES: usize = 20;

#[test]
fn eight_writers_eight_readers_lose_nothing() {
    let d = TempDir::new("storm");
    let brain = init_brain(&d);
    for i in 0..40 {
        std::fs::write(
            brain.join(format!("notes/fundo{i}.md")),
            format!("# Fundo {i}\nmaterial comum de fundo\n"),
        )
        .unwrap();
    }
    let done = std::sync::atomic::AtomicUsize::new(0);
    let mut written: BTreeSet<String> = BTreeSet::new();
    thread::scope(|s| {
        let mut handles = Vec::new();
        for w in 0..WRITERS {
            let (home, brain, done) = (d.path(), &brain, &done);
            handles.push(s.spawn(move || {
                let mut m = Mcp::spawn(home, brain, &["--client", "claude-code"]);
                let mut tokens = Vec::new();
                for i in 0..WRITES {
                    let token = format!("tok{w}x{i}z");
                    let r = m.call(
                        "memory_write",
                        json!({"key": token, "type": "concept", "title": format!("Assunto {token}"), "summary": format!("Registro {token} do escritor {w}, item {}.", i * 17 + w * 1000)}),
                    );
                    assert_eq!(r["outcome"], "written", "{r}");
                    // ler o que escrevi, no mesmo processo
                    let f = m.call("memory_search", json!({"query": token}));
                    assert_eq!(f["total"], 1, "ryw {token}: {f}");
                    tokens.push(token);
                }
                done.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                assert!(m.close().success(), "sem pânico");
                tokens
            }));
        }
        for r in 0..READERS {
            let (home, brain, done) = (d.path(), &brain, &done);
            s.spawn(move || {
                let mut m = Mcp::spawn(home, brain, &["--client", "claude-code"]);
                let mut n = 0;
                while done.load(std::sync::atomic::Ordering::SeqCst) < WRITERS {
                    let f = m.call(
                        "memory_search",
                        json!({"query": ["material comum de fundo", "registro"], "limit": 3}),
                    );
                    assert_ne!(f["status"], "no_match", "leitor {r}: brain nunca some: {f}");
                    assert!(f["coverage"]["notes"].as_u64().unwrap() >= 40, "{f}");
                    n += 1;
                }
                assert!(n > 0);
                assert!(m.close().success(), "sem pânico");
            });
        }
        for h in handles {
            written.extend(h.join().unwrap());
        }
    });
    assert_eq!(written.len(), WRITERS * WRITES);
    // Controle positivo: confere no disco e por um processo novo, sem índice aproveitado.
    let files = std::fs::read_dir(brain.join("memories/concept"))
        .unwrap()
        .count();
    assert_eq!(files, written.len());
    std::fs::remove_file(brain.join(".omnimind/snapshot.bin")).ok();
    let mut m = Mcp::spawn(d.path(), &brain, &["--client", "claude-code"]);
    for t in &written {
        let f = m.call("memory_search", json!({"query": t}));
        assert_eq!(f["total"], 1, "{t}: {f}");
    }
    let log = run(
        d.path(),
        &["doctor", "--brain", brain.to_str().unwrap(), "--json"],
    );
    let v: serde_json::Value = serde_json::from_slice(&log.stdout).unwrap();
    assert_eq!(v["health"]["journal_bad_lines"], 0, "{v}");
    assert_eq!(v["health"]["memories"], written.len());
}

#[test]
fn hooks_and_cli_writers_share_a_session_segment() {
    let d = TempDir::new("segment");
    let brain = init_brain(&d);
    thread::scope(|s| {
        for t in 0..8 {
            let (home, brain) = (d.path(), &brain);
            s.spawn(move || {
                for i in 0..15 {
                    let input =
                        json!({"session_id": "mesma", "prompt": format!("não, número {t} {i}")});
                    let mut c = cmd(home)
                        .args(["hook", "prompt", "--brain", brain.to_str().unwrap()])
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::null())
                        .spawn()
                        .unwrap();
                    use std::io::Write;
                    c.stdin
                        .take()
                        .unwrap()
                        .write_all(input.to_string().as_bytes())
                        .unwrap();
                    assert!(c.wait().unwrap().success());
                }
            });
        }
    });
    let log = run(
        d.path(),
        &[
            "log",
            "--brain",
            brain.to_str().unwrap(),
            "--json",
            "--limit",
            "1000",
        ],
    );
    let v: serde_json::Value = serde_json::from_slice(&log.stdout).unwrap();
    assert_eq!(v["bad_lines"], 0);
    let n = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["ev"] == "signal")
        .count();
    assert_eq!(n, 120);
}
