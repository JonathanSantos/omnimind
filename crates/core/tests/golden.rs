//! Golden set: recall@5 por classe, nos brains de exemplo copiados.

mod common;

use common::TempDir;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use omnimind_core::apply_norm_title as norm;
use omnimind_core::{Brain, SearchQuery, SearchStatus};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

struct Case {
    class: String,
    question: String,
    expected: Vec<String>,
    forbidden: Option<String>,
}

fn parse(path: &Path) -> Vec<Case> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut class = String::new();
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some(c) = line.strip_prefix("## ") {
            class = c.trim().to_string();
        } else if let Some(item) = line.strip_prefix("- ") {
            let (q, rest) = item.split_once(" -> ").unwrap_or((item, ""));
            let (exp, forb) = rest
                .split_once(" || ")
                .map_or((rest, None), |(a, b)| (a, Some(b.trim().to_string())));
            let expected = exp
                .split(';')
                .map(|e| e.trim().to_string())
                .filter(|e| !e.is_empty())
                .collect();
            out.push(Case {
                class: class.clone(),
                question: q.trim().to_string(),
                expected,
                forbidden: forb,
            });
        }
    }
    out
}

fn imported(name: &str, d: &TempDir) -> Brain {
    let b = Brain::init(d.join(name), name).unwrap();
    b.import_markdown(&fixtures().join("brains").join(name))
        .unwrap();
    b
}

#[derive(Default)]
struct Score {
    hit: usize,
    total: usize,
    strong: usize,
    misses: Vec<String>,
}

fn evaluate(b: &Brain, cases: &[Case], scores: &mut BTreeMap<String, Score>, report_misses: bool) {
    for c in cases {
        let r = b.search(&SearchQuery::new(&c.question)).unwrap();
        let titles: Vec<String> = r.hits.iter().map(|h| norm(&h.title)).collect();
        let s = scores.entry(c.class.clone()).or_default();
        s.total += 1;
        let ok = match c.class.as_str() {
            "Abstenção" => matches!(r.status, SearchStatus::NoMatch | SearchStatus::Weak),
            "Atualização" => {
                let want = norm(&c.expected[0]);
                let pos = titles.iter().position(|t| t == &want);
                // A versão velha não pode vencer: ou some, ou vem depois, ou vem com aviso.
                let bad = c.forbidden.as_deref().map(norm);
                let bad_pos = bad.and_then(|b| titles.iter().position(|t| *t == b));
                let bad_active = bad_pos.is_some_and(|p| r.hits[p].warning.is_none());
                pos.is_some() && (bad_pos.is_none() || !bad_active || pos < bad_pos)
            }
            _ => c.expected.iter().any(|e| titles.contains(&norm(e))),
        };
        if ok {
            s.hit += 1;
            s.strong += usize::from(r.status == SearchStatus::Ok);
        } else if report_misses {
            s.misses.push(format!(
                "{} => {:?} {:?}",
                c.question,
                r.status,
                titles.iter().take(3).collect::<Vec<_>>()
            ));
        }
    }
}

fn run(name: &str, scores: &mut BTreeMap<String, Score>, heldout: &mut BTreeMap<String, Score>) {
    let d = TempDir::new("golden");
    let b = imported(name, &d);
    let f = fixtures();
    let mut cases = parse(&f.join("brains").join(name).join("schema/evals.md"));
    cases.extend(parse(&f.join("evals").join(format!("{name}.md"))));
    evaluate(&b, &cases, scores, true);
    // Perguntas que o autor do índice não viu: só o agregado aparece.
    let held = parse(&f.join("evals").join(format!("heldout-{name}.md")));
    evaluate(&b, &held, heldout, false);
}

#[test]
fn golden_recall_by_class() {
    let mut per_brain = BTreeMap::new();
    let mut all: BTreeMap<String, Score> = BTreeMap::new();
    let mut held: BTreeMap<String, Score> = BTreeMap::new();
    for name in ["personal", "work"] {
        let mut s = BTreeMap::new();
        run(name, &mut s, &mut held);
        for (k, v) in &s {
            let e = all.entry(k.clone()).or_default();
            e.hit += v.hit;
            e.total += v.total;
            e.strong += v.strong;
            e.misses.extend(v.misses.iter().cloned());
        }
        per_brain.insert(name, s);
    }
    for (name, s) in &per_brain {
        for (class, v) in s {
            println!(
                "{name:9} {class:12} {}/{} = {:.3}",
                v.hit,
                v.total,
                v.hit as f32 / v.total as f32
            );
        }
    }
    for (class, v) in &all {
        println!(
            "TOTAL     {class:12} {}/{} = {:.3}",
            v.hit,
            v.total,
            v.hit as f32 / v.total as f32
        );
        for m in &v.misses {
            println!("    miss: {m}");
        }
    }
    for (class, v) in &held {
        println!(
            "HELD-OUT  {class:12} {}/{} = {:.3} (acertos com status ok: {})",
            v.hit,
            v.total,
            v.hit as f32 / v.total as f32,
            v.strong
        );
    }
    let rate = |c: &str| all.get(c).map_or(0.0, |v| v.hit as f32 / v.total as f32);
    assert!(all.len() >= 5, "golden set incompleto: {:?}", all.keys());
    assert!(rate("Direta") >= 1.0, "direta");
    assert!(rate("Morfologia") >= 1.0, "morfologia");
    assert!(rate("Atualização") >= 1.0, "atualização");
    assert!(rate("Abstenção") >= 0.8, "abstenção");
    assert!(rate("Paráfrase") >= 0.7, "paráfrase");
    for (class, v) in &held {
        assert!(v.hit as f32 / v.total as f32 >= 0.7, "held-out {class}");
    }
}

#[test]
fn reimport_adds_nothing() {
    let d = TempDir::new("reimport");
    let b = imported("work", &d);
    let before = common::hash_tree(b.root());
    let tags_before = b.health().unwrap().tags;
    let r = b.import_markdown(&fixtures().join("brains/work")).unwrap();
    assert_eq!(common::hash_tree(b.root()), before);
    assert_eq!(b.health().unwrap().tags, tags_before);
    assert_eq!(r.notes, 0, "{r:?}");
}

#[test]
fn imported_traps_survive() {
    let d = TempDir::new("traps");
    let b = imported("work", &d);
    let mems = b.memories().unwrap();
    let state = |needle: &str| {
        mems.iter()
            .filter(|m| m.title.contains(needle))
            .map(|m| (m.title.clone(), m.state))
            .collect::<Vec<_>>()
    };
    let deploy = state("Deploy do otimizador");
    assert_eq!(deploy.len(), 2);
    assert!(
        deploy
            .iter()
            .any(|(t, s)| t.ends_with("(janela e canary)")
                && *s == omnimind_core::MemoryState::Superseded),
        "{deploy:?}"
    );
    let r = b.search(&SearchQuery::new("ACME-WRK-9F3X")).unwrap();
    assert!(!r.hits.is_empty(), "âncora preservada");
    assert!(
        r.hits
            .iter()
            .any(|h| h.mem_type == Some(omnimind_core::MemoryType::Decision)),
        "{:?}",
        r.hits
    );
}
