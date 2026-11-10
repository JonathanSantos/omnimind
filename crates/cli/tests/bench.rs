//! Benchmark com 5.000 notas fictícias. Roda à parte, em release:
//! `cargo test --release -p omnimind-cli --test bench -- --ignored --nocapture`.
//! Sai com falha quando estoura uma meta.

mod common;

use common::*;
use serde_json::json;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    /// Frequência tipo Zipf, como texto real.
    fn zipf(&mut self, n: usize) -> usize {
        let u = (self.next() % 1_000_000) as f64 / 1_000_000.0;
        ((n as f64).powf(u) as usize).min(n - 1)
    }
}

const SYL: &[&str] = &[
    "ba", "ca", "da", "fa", "ga", "la", "ma", "na", "pa", "ra", "sa", "ta", "va", "be", "ce", "de",
    "fe", "le", "me", "ne", "pe", "re", "se", "te", "bi", "ci", "di", "fi", "li", "mi", "ni", "pi",
    "ri", "si", "ti", "bo", "co", "do", "fo", "lo", "mo", "no", "po", "ro", "so", "to", "cao",
    "coes", "dade", "mento",
];

fn vocabulary(r: &mut Rng, n: usize) -> Vec<String> {
    (0..n)
        .map(|_| {
            let k = 2 + r.below(3);
            (0..k).map(|_| SYL[r.below(SYL.len())]).collect::<String>()
        })
        .collect()
}

fn pct(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(f64::total_cmp);
    v[((v.len() as f64 - 1.0) * p).round() as usize]
}

fn rss_mb(pid: u32) -> f64 {
    let o = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stdout)
        .trim()
        .parse::<f64>()
        .unwrap_or(0.0)
        / 1024.0
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn hook_ms(
    home: &std::path::Path,
    brain: &std::path::Path,
    ev: &str,
    input: &str,
    n: usize,
) -> (f64, f64) {
    let mut v = Vec::new();
    for _ in 0..n {
        let t = Instant::now();
        let mut c = cmd(home)
            .args(["hook", ev, "--brain", brain.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        c.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        c.wait().unwrap();
        v.push(ms(t));
    }
    (pct(&mut v, 0.5), pct(&mut v, 0.95))
}

#[test]
#[ignore = "benchmark: rode em release"]
fn bench_5000_notes() {
    let d = TempDir::new("bench");
    let brain = init_brain(&d);
    let mut r = Rng(0x5EED_1234_ABCD);
    let vocab = vocabulary(&mut r, 20_000);
    let t = Instant::now();
    let mut tokens = Vec::new();
    for i in 0..5000 {
        let dir = brain.join(format!("notes/area{}/sub{}", i % 40, i % 7));
        std::fs::create_dir_all(&dir).unwrap();
        let mut text = format!("# Nota {i} {}\n\n", vocab[r.zipf(vocab.len())]);
        while text.len() < 3000 {
            for _ in 0..12 {
                text.push_str(&vocab[r.zipf(vocab.len())]);
                text.push(' ');
            }
            text.push('\n');
        }
        let tok = format!("unico{i}token");
        text.push_str(&format!("{tok} [[Nota {}]]\n", (i + 1) % 5000));
        tokens.push((format!("notes/area{}/sub{}/n{i}.md", i % 40, i % 7), tok));
        std::fs::write(dir.join(format!("n{i}.md")), text).unwrap();
    }
    for i in 0..500 {
        let id = gen_id(&mut r);
        let text = format!(
            "---\nid: {id}\ntype: concept\ntitle: Memória {i} {}\nsummary: Resumo {} {}\nauthority: agent\ncreated: 2026-10-31\nupdated: 2026-10-31\n---\ncorpo {} {} {}\n",
            vocab[r.zipf(vocab.len())], vocab[r.zipf(vocab.len())], vocab[r.zipf(vocab.len())],
            vocab[r.zipf(vocab.len())], vocab[r.zipf(vocab.len())], vocab[r.zipf(vocab.len())],
        );
        std::fs::write(brain.join(format!("memories/concept/m{i}.md")), text).unwrap();
    }
    println!("gerar arquivos: {:.0} ms", ms(t));

    let b = brain.to_str().unwrap();
    let t = Instant::now();
    assert!(run(d.path(), &["doctor", "--brain", b]).status.success());
    let open_cold = ms(t);
    let snap = brain.join(".omnimind/snapshot.bin");
    assert!(snap.exists(), "doctor grava instantâneo");
    println!("abertura sem índice (doctor): {open_cold:.0} ms");
    println!(
        "instantâneo: {:.1} MB",
        std::fs::metadata(&snap).unwrap().len() as f64 / 1e6
    );
    std::thread::sleep(Duration::from_millis(2100));

    let mut opens = Vec::new();
    for _ in 0..9 {
        let t = Instant::now();
        assert!(run(
            d.path(),
            &["search", "--brain", b, "--limit", "1", &vocab[10]]
        )
        .status
        .success());
        opens.push(ms(t));
    }
    let open_warm = pct(&mut opens, 0.5);
    println!("abertura com instantâneo + busca (processo inteiro): p50 {open_warm:.1} ms");

    let t = Instant::now();
    let mut m = Mcp::spawn(d.path(), &brain, &["--client", "claude-code"]);
    m.request(
        "initialize",
        json!({"protocolVersion": "2025-11-25", "clientInfo": {"name": "claude-code"}}),
    );
    let init_ms = ms(t);
    let f = m.call("memory_search", json!({"query": vocab[3]}));
    assert_eq!(f["status"], "ok", "{f}");
    println!(
        "initialize: {init_ms:.1} ms; primeira busca pronta em {:.1} ms",
        ms(t)
    );

    // Busca isolada: cada uma com a varredura de stat, como um agente real.
    let mut spaced = Vec::new();
    for i in 0..60 {
        std::thread::sleep(Duration::from_millis(260));
        let q = format!(
            "{} {}",
            vocab[r.zipf(vocab.len())],
            vocab[r.below(vocab.len())]
        );
        let t = Instant::now();
        m.call("memory_search", json!({"query": q}));
        if i > 0 {
            spaced.push(ms(t));
        }
    }
    let (s50, s95) = (pct(&mut spaced, 0.5), pct(&mut spaced, 0.95));
    println!("busca com varredura (a cada 260 ms): p50 {s50:.1} ms, p95 {s95:.1} ms");

    let mut burst = Vec::new();
    for _ in 0..400 {
        let q = json!([
            vocab[r.zipf(vocab.len())],
            format!("{} {}", vocab[r.zipf(500)], vocab[r.zipf(500)])
        ]);
        let t = Instant::now();
        m.call("memory_search", json!({"query": q}));
        burst.push(ms(t));
    }
    let (b50, b95) = (pct(&mut burst, 0.5), pct(&mut burst, 0.95));
    println!("busca em rajada, 2 reformulações: p50 {b50:.1} ms, p95 {b95:.1} ms");

    let mut after_write = Vec::new();
    let mut write_ms = Vec::new();
    for i in 0..38 {
        let tok = format!("escrita{i}marca");
        let t = Instant::now();
        let w = m.call("memory_write", json!({"key": tok, "type": "decision", "title": format!("Decisão {tok}"), "summary": format!("Fato {tok} para medir.")}));
        write_ms.push(ms(t));
        assert_eq!(w["outcome"], "written", "{w}");
        let t = Instant::now();
        let f = m.call("memory_search", json!({"query": tok}));
        after_write.push(ms(t));
        assert_eq!(f["total"], 1);
    }
    let (w50, w95) = (pct(&mut after_write, 0.5), pct(&mut after_write, 0.95));
    let (wr50, wr95) = (pct(&mut write_ms, 0.5), pct(&mut write_ms, 0.95));
    println!("escrita: p50 {wr50:.1} ms, p95 {wr95:.1} ms");
    println!("1ª busca após escrita, mesmo processo: p50 {w50:.1} ms, p95 {w95:.1} ms");

    let mut other = Mcp::spawn(d.path(), &brain, &["--client", "claude-code"]);
    other.call("memory_search", json!({"query": "aquecer"}));
    let mut writer = Mcp::spawn(d.path(), &brain, &["--client", "claude-code"]);
    let mut cross = Vec::new();
    for i in 0..30 {
        let tok = format!("cruzada{i}marca");
        let w = writer.call("memory_write", json!({"key": tok, "type": "decision", "title": format!("Decisão {tok}"), "summary": format!("Fato {tok} para medir.")}));
        assert_eq!(w["outcome"], "written", "{w}");
        let t = Instant::now();
        let f = other.call("memory_search", json!({"query": tok}));
        cross.push(ms(t));
        assert_eq!(f["total"], 1, "outro processo vê na hora");
    }
    let (c50, c95) = (pct(&mut cross, 0.5), pct(&mut cross, 0.95));
    println!(
        "1ª busca após escrita, outro processo: p50 {c50:.1} ms, p95 {c95:.1} ms, máx {:.1} ms",
        pct(&mut cross, 1.0)
    );

    std::thread::sleep(Duration::from_millis(500));
    let rss = rss_mb(m.child.id());
    let rss2 = rss_mb(other.child.id());
    println!("RSS em repouso: {rss:.1} MB (em uso) / {rss2:.1} MB (novo)");

    // Oráculo no brain grande: token único por nota, contra o que está no disco.
    let mut o = Rng(99);
    for _ in 0..200 {
        let (path, tok) = &tokens[o.below(tokens.len())];
        let f = m.call("memory_search", json!({"query": tok, "limit": 3}));
        assert_eq!(f["total"], 1, "{tok}");
        assert_eq!(f["results"][0]["id"], format!("note:{path}"));
    }
    println!("oráculo 200 tokens em 5.000 notas: ok");

    let s = json!({"session_id": "bench", "cwd": "/tmp/p"}).to_string();
    let (h50, h95) = hook_ms(d.path(), &brain, "session-start", &s, 30);
    println!("hook session-start (560 memórias): p50 {h50:.1} ms, p95 {h95:.1} ms");
    let p = json!({"session_id": "bench", "prompt": "não, faz diferente"}).to_string();
    let (p50, p95) = hook_ms(d.path(), &brain, "prompt", &p, 30);
    println!("hook prompt: p50 {p50:.1} ms, p95 {p95:.1} ms");

    let bin = std::fs::metadata(BIN).unwrap().len() as f64 / 1e6;
    println!("binário: {bin:.1} MB");

    assert!(s95 <= 100.0, "busca p95 {s95}");
    assert!(w95 <= 100.0, "1ª busca após escrita p95 {w95}");
    assert!(c95 <= 100.0, "1ª busca em outro processo p95 {c95}");
    assert!(rss <= 150.0, "rss {rss}");
    assert!(h95 <= 50.0, "hook {h95}");
    assert!(p95 <= 50.0, "hook {p95}");
}

fn gen_id(r: &mut Rng) -> String {
    const C: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    (0..26)
        .map(|i| if i == 0 { '0' } else { C[r.below(32)] as char })
        .collect()
}
