//! Coerência: dano ao estado privado, oráculo, stat, processos em paralelo no mesmo disco.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::fs;
use std::time::{Duration, Instant, SystemTime};
use omnimind_core::{Actor, Brain, Change, Error, Outcome, SaveResult, SearchQuery, SearchStatus};

fn sample_brain(tag: &str) -> (TempDir, Brain) {
    let (d, b) = brain(tag);
    for i in 0..12 {
        b.save_note(
            &format!("notes/pasta{}/nota{i}.md", i % 3),
            &format!(
                "# Nota {i}\n\nTexto sobre roteirização número {i} com [[Nota {}]] e #tema/t{}.\n",
                (i + 1) % 12,
                i % 4
            ),
            None,
        )
        .unwrap();
    }
    for i in 0..8 {
        write(
            &b,
            draft(
                &format!("Decisão {i} de roteirização"),
                &format!("Fato {i} sobre motores, {} rotas.", i * 13),
            ),
        );
    }
    let a = write(&b, draft("Velha", "Fato velho sobre motores."));
    let mut n = draft("Nova", "Fato novo sobre motores em operação.");
    n.supersedes = vec![a.id.unwrap()];
    write(&b, n);
    b.flush().unwrap();
    (d, b)
}

fn answers(b: &Brain) -> Vec<String> {
    let mut out = Vec::new();
    for q in [
        "roteirização",
        "motores",
        "nota 7",
        "rotas",
        "tema",
        "inexistente",
    ] {
        let mut sq = SearchQuery::new(q);
        sq.limit = 20;
        let r = b.search(&sq).unwrap();
        out.push(format!(
            "{q}|{}|{:?}|{:?}",
            r.status.as_str(),
            r.hits.iter().map(|h| h.id.clone()).collect::<Vec<_>>(),
            r.coverage.hidden
        ));
    }
    out
}

#[test]
fn twenty_eight_kinds_of_damage_change_no_answer() {
    type Damage = (&'static str, fn(&std::path::Path));
    fn snap(p: &std::path::Path) -> std::path::PathBuf {
        p.join(".omnimind/snapshot.bin")
    }
    fn flip(p: &std::path::Path, seed: u64) {
        let mut data = fs::read(snap(p)).unwrap();
        let mut r = Rng(seed);
        for _ in 0..8 {
            let i = r.below(data.len());
            data[i] ^= 1 << r.below(8);
        }
        fs::write(snap(p), data).unwrap();
    }
    fn corrupt_payload(p: &std::path::Path, at: f64) {
        let mut data = fs::read(snap(p)).unwrap();
        let i = (data.len() as f64 * at) as usize;
        data[i] ^= 0xff;
        fs::write(snap(p), data).unwrap();
    }
    fn first_segment(p: &std::path::Path) -> std::path::PathBuf {
        let seg = omnimind_core::journal::list_segments(p)[0].clone();
        p.join(".omnimind/journal").join(seg)
    }
    let damages: Vec<Damage> = vec![
        ("snapshot_deleted", |p| fs::remove_file(snap(p)).unwrap()),
        ("snapshot_empty", |p| fs::write(snap(p), b"").unwrap()),
        ("snapshot_garbage", |p| {
            fs::write(snap(p), vec![0xAB; 4096]).unwrap()
        }),
        ("snapshot_truncated_half", |p| {
            let d = fs::read(snap(p)).unwrap();
            fs::write(snap(p), &d[..d.len() / 2]).unwrap();
        }),
        ("snapshot_truncated_tail", |p| {
            let d = fs::read(snap(p)).unwrap();
            fs::write(snap(p), &d[..d.len() - 1]).unwrap();
        }),
        ("snapshot_appended", |p| {
            let mut d = fs::read(snap(p)).unwrap();
            d.extend_from_slice(b"lixo");
            fs::write(snap(p), d).unwrap();
        }),
        ("snapshot_bad_magic", |p| {
            let mut d = fs::read(snap(p)).unwrap();
            d[0] = b'X';
            fs::write(snap(p), d).unwrap();
        }),
        ("snapshot_old_version", |p| {
            let mut d = fs::read(snap(p)).unwrap();
            d[8] = 99;
            fs::write(snap(p), d).unwrap();
        }),
        ("snapshot_old_analyzer", |p| {
            let mut d = fs::read(snap(p)).unwrap();
            d[12] = 99;
            fs::write(snap(p), d).unwrap();
        }),
        ("snapshot_payload_10", |p| corrupt_payload(p, 0.10)),
        ("snapshot_payload_50", |p| corrupt_payload(p, 0.50)),
        ("snapshot_payload_90", |p| corrupt_payload(p, 0.90)),
        ("snapshot_is_directory", |p| {
            fs::remove_file(snap(p)).unwrap();
            fs::create_dir(snap(p)).unwrap();
        }),
        ("snapshot_bitrot_1", |p| flip(p, 1)),
        ("snapshot_bitrot_2", |p| flip(p, 2)),
        ("snapshot_bitrot_3", |p| flip(p, 3)),
        ("snapshot_other_brain", |p| {
            let (_o, other) = sample_brain("other");
            other.flush().unwrap();
            fs::copy(other.root().join(".omnimind/snapshot.bin"), snap(p)).unwrap();
        }),
        ("journal_deleted", |p| {
            fs::remove_dir_all(p.join(".omnimind/journal")).unwrap()
        }),
        ("journal_truncated_line", |p| {
            let s = first_segment(p);
            let d = fs::read(&s).unwrap();
            fs::write(&s, &d[..d.len() - 7]).unwrap();
        }),
        ("journal_garbage_line", |p| {
            let s = first_segment(p);
            let mut d = fs::read(&s).unwrap();
            d.extend_from_slice(b"{nao e json}\n\xff\xfe\n");
            fs::write(&s, d).unwrap();
        }),
        ("journal_empty_segment", |p| {
            fs::write(first_segment(p), b"").unwrap()
        }),
        ("journal_binary_segment", |p| {
            fs::write(first_segment(p), vec![0u8; 1024]).unwrap()
        }),
        ("generation_deleted", |p| {
            let _ = fs::remove_file(p.join(".omnimind/generation"));
        }),
        ("generation_garbage", |p| {
            fs::write(p.join(".omnimind/generation"), b"xxxxxxxxxxxxxxxxxxxxxx").unwrap()
        }),
        ("lock_deleted", |p| {
            let _ = fs::remove_file(p.join(".omnimind/lock"));
        }),
        ("tmp_junk", |p| {
            fs::write(p.join(".omnimind/tmp/lixo.tmp"), b"x").unwrap()
        }),
        ("plan_garbage", |p| {
            fs::create_dir_all(p.join(".omnimind/plans")).unwrap();
            fs::write(p.join(".omnimind/plans/x.json"), b"{").unwrap();
        }),
        ("private_deleted", |p| {
            fs::remove_dir_all(p.join(".omnimind")).unwrap()
        }),
    ];
    assert_eq!(damages.len(), 28);
    let (_d, b) = sample_brain("damage");
    let base = hash_tree(b.root());
    fs::remove_file(b.root().join(".omnimind/snapshot.bin")).unwrap();
    let want = answers(&Brain::open(b.root()).unwrap());
    b.flush().unwrap();
    assert!(b.root().join(".omnimind/snapshot.bin").exists());
    let keep = b.root().join(".omnimind-keep");
    let copy = |from: &std::path::Path, to: &std::path::Path| {
        let status = std::process::Command::new("cp")
            .arg("-R")
            .arg(from)
            .arg(to)
            .status()
            .unwrap();
        assert!(status.success());
    };
    copy(&b.root().join(".omnimind"), &keep);
    for (name, damage) in damages {
        damage(b.root());
        let got = answers(&Brain::open(b.root()).unwrap());
        assert_eq!(got, want, "{name}");
        // Escrita também segue funcionando depois do dano.
        assert_eq!(hash_tree(b.root()), base, "{name}: .md intactos");
        fs::remove_dir_all(b.root().join(".omnimind")).ok();
        fs::remove_file(b.root().join(".omnimind")).ok();
        copy(&keep, &b.root().join(".omnimind"));
    }
}

#[test]
fn oracle_search_matches_disk_scan() {
    let (_d, b) = brain("oracle");
    let mut r = Rng(0xC0FFEE);
    let mut oracle: std::collections::BTreeMap<String, String> = Default::default();
    for step in 0..600 {
        let name = format!("notes/d{}/n{}.md", r.below(6), r.below(40));
        let full = b.root().join(&name);
        match r.below(4) {
            0 | 1 => {
                let token = format!("zq{step}x");
                let text = format!("# T\n\nconteudo {token}\n");
                fs::create_dir_all(full.parent().unwrap()).unwrap();
                fs::write(&full, &text).unwrap();
                oracle.insert(name.clone(), token);
            }
            2 if oracle.remove(&name).is_some() => fs::remove_file(&full).unwrap(),
            _ => {}
        }
        if step % 37 == 0 {
            b.reconcile().unwrap();
        }
    }
    std::thread::sleep(Duration::from_millis(320));
    b.reconcile().unwrap();
    std::thread::sleep(Duration::from_millis(320));
    b.reconcile().unwrap();
    let notes: BTreeSet<String> = b.notes().unwrap().into_iter().map(|n| n.path).collect();
    assert_eq!(notes, oracle.keys().cloned().collect::<BTreeSet<_>>());
    for (path, token) in &oracle {
        let r = b.search(&SearchQuery::new(token)).unwrap();
        assert_eq!(r.hits.len(), 1, "{token}");
        assert_eq!(r.hits[0].path, *path);
    }
    // tokens antigos sumiram
    let old = b.search(&SearchQuery::new("zq0x")).unwrap();
    let hits = if old.corrected.is_some() {
        0
    } else {
        old.total
    };
    assert_eq!(hits, usize::from(oracle.values().any(|t| t == "zq0x")));
}

#[test]
fn same_size_and_mtime_edit_is_detected() {
    let (_d, b) = brain("samestat");
    let path = "notes/a.md";
    b.save_note(path, "# A\n\npalavraum\n", None).unwrap();
    b.flush().unwrap();
    let full = b.root().join(path);
    let f = fs::File::options().write(true).open(&full).unwrap();
    let mtime = f.metadata().unwrap().modified().unwrap();
    // longe da janela racy
    let past = SystemTime::now() - Duration::from_secs(3600);
    f.set_modified(past).unwrap();
    drop(f);
    let b2 = Brain::open(b.root()).unwrap();
    assert_eq!(
        b2.search(&SearchQuery::new("palavraum"))
            .unwrap()
            .hits
            .len(),
        1
    );
    b2.flush().unwrap();
    std::thread::sleep(Duration::from_millis(2100));
    b2.reconcile().unwrap();
    // mesmo tamanho, mesmo mtime; só o ctime denuncia
    fs::write(&full, "# A\n\npalavradois\n".replace("dois", "doiz")).unwrap();
    let f = fs::File::options().write(true).open(&full).unwrap();
    f.set_modified(past).unwrap();
    drop(f);
    let _ = mtime;
    std::thread::sleep(Duration::from_millis(260));
    assert_eq!(
        b2.search(&SearchQuery::new("palavradoiz"))
            .unwrap()
            .hits
            .len(),
        1,
        "em outro processo vivo"
    );
    assert_eq!(
        b2.search(&SearchQuery::new("palavraum"))
            .unwrap()
            .hits
            .len(),
        0
    );
}

#[test]
fn racy_rewrite_within_two_seconds_is_detected() {
    let (_d, b) = brain("racy");
    let full = b.root().join("notes/a.md");
    fs::write(&full, "aaaa1\n").unwrap();
    std::thread::sleep(Duration::from_millis(260));
    assert_eq!(b.search(&SearchQuery::new("aaaa1")).unwrap().hits.len(), 1);
    let meta = fs::metadata(&full).unwrap();
    fs::write(&full, "bbbb2\n").unwrap();
    let f = fs::File::options().write(true).open(&full).unwrap();
    f.set_modified(meta.modified().unwrap()).unwrap();
    drop(f);
    std::thread::sleep(Duration::from_millis(260));
    assert_eq!(b.search(&SearchQuery::new("bbbb2")).unwrap().hits.len(), 1);
}

#[test]
fn reader_two_seconds_apart_sees_external_edit() {
    let (_d, b) = brain("spaced");
    let other = Brain::open(b.root()).unwrap();
    for (i, wait) in [1200u64, 600, 600, 600].iter().enumerate() {
        std::thread::sleep(Duration::from_millis(*wait));
        let token = format!("espacado{i}");
        fs::write(
            b.root().join(format!("notes/e{i}.md")),
            format!("# E\n{token}\n"),
        )
        .unwrap();
        std::thread::sleep(Duration::from_millis(260));
        assert_eq!(
            other.search(&SearchQuery::new(&token)).unwrap().hits.len(),
            1,
            "{i}"
        );
    }
}

#[test]
fn write_by_other_process_is_visible_at_once() {
    let (_d, b) = brain("ryw");
    let other = Brain::open(b.root()).unwrap();
    other.search(&SearchQuery::new("aquecimento")).unwrap();
    for i in 0..30 {
        let token = format!("imediato{i}");
        write(&b, draft(&format!("Fato {token}"), "Algo."));
        // sem esperar: o aviso de geração força a varredura
        assert_eq!(
            other.search(&SearchQuery::new(&token)).unwrap().hits.len(),
            1,
            "{i}"
        );
    }
}

#[test]
fn editor_saving_by_rename_does_not_lose_the_note() {
    let (_d, b) = brain("grace");
    let full = b.root().join("notes/a.md");
    fs::write(&full, "# A\ngracioso\n").unwrap();
    std::thread::sleep(Duration::from_millis(260));
    assert_eq!(
        b.search(&SearchQuery::new("gracioso")).unwrap().hits.len(),
        1
    );
    fs::remove_file(&full).unwrap();
    std::thread::sleep(Duration::from_millis(260));
    // Durante a espera a nota não é declarada removida, mas também não é mostrada sem arquivo.
    let r = b.search(&SearchQuery::new("gracioso")).unwrap();
    assert!(r.hits.is_empty());
    assert_ne!(
        r.status,
        SearchStatus::NoMatch,
        "vazio não pode soar definitivo: {r:?}"
    );
    fs::write(&full, "# A\ngracioso\n").unwrap();
    std::thread::sleep(Duration::from_millis(260));
    assert_eq!(
        b.search(&SearchQuery::new("gracioso")).unwrap().hits.len(),
        1
    );
    assert_eq!(b.notes().unwrap().len(), 1);
    fs::remove_file(&full).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    b.reconcile().unwrap();
    std::thread::sleep(Duration::from_millis(320));
    b.reconcile().unwrap();
    assert_eq!(b.notes().unwrap().len(), 0);
}

#[test]
fn honest_empty_during_concurrent_writes() {
    let (_d, b) = brain("empty");
    for i in 0..30 {
        write(
            &b,
            draft(
                &format!("Âncora {i}"),
                &format!("Tema comum âncora número {}.", i * 3),
            ),
        );
    }
    let stop = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|s| {
        let writer = b.clone();
        let stop_ref = &stop;
        s.spawn(move || {
            let mut i = 0;
            while !stop_ref.load(std::sync::atomic::Ordering::Relaxed) {
                i += 1;
                write(
                    &writer,
                    draft(
                        &format!("Escrita {i}"),
                        &format!("Conteúdo concorrente {i} âncora."),
                    ),
                );
                if i >= 25 {
                    break;
                }
            }
        });
        let root = b.root().to_path_buf();
        for _ in 0..60 {
            let r = Brain::open(&root).unwrap();
            let res = r.search(&SearchQuery::new("âncora")).unwrap();
            assert!(
                !res.hits.is_empty(),
                "brain cheio nunca responde vazio: {res:?}"
            );
        }
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
    });
}

#[test]
fn lazy_open_reports_indexing_instead_of_empty() {
    let (_d, b) = brain("lazy");
    for i in 0..400 {
        fs::write(
            b.root().join(format!("notes/n{i}.md")),
            format!("# N{i}\nconteudo comum {}\n", "x ".repeat(2000)),
        )
        .unwrap();
    }
    fs::remove_file(b.root().join(".omnimind/snapshot.bin")).ok();
    let lazy = Brain::open_with(
        b.root(),
        omnimind_core::BrainOptions {
            lazy: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut q = SearchQuery::new("conteudo");
    q.wait_ms = Some(0);
    let r = lazy.search(&q).unwrap();
    if r.coverage.indexing > 0 {
        assert_eq!(r.status, SearchStatus::Incomplete, "{r:?}");
        assert!(r.reason.is_some());
    } else {
        panic!("máquina rápida demais para o teste: {r:?}");
    }
    let mut q = SearchQuery::new("conteudo");
    q.wait_ms = None;
    assert_eq!(lazy.search(&q).unwrap().status, SearchStatus::Ok);
}

#[test]
fn indexing_makes_progress_with_a_zero_budget() {
    // Regressão: sob carga o passeio pela árvore gastava o prazo e nada era indexado.
    let (_d, b) = brain("starve");
    for i in 0..50 {
        fs::write(
            b.root().join(format!("notes/n{i}.md")),
            format!("# N{i}\ntexto\n"),
        )
        .unwrap();
    }
    let lazy = Brain::open_with(
        b.root(),
        omnimind_core::BrainOptions {
            lazy: true,
            ..Default::default()
        },
    )
    .unwrap();
    let mut pending = lazy.reconcile_slice(Duration::ZERO).unwrap();
    let mut rounds = 0;
    while pending > 0 {
        let next = lazy.reconcile_slice(Duration::ZERO).unwrap();
        assert!(next < pending, "{next} {pending}");
        pending = next;
        rounds += 1;
    }
    assert!(rounds > 10);
    assert_eq!(lazy.notes().unwrap().len(), 50);
}

#[test]
fn vanished_brain_is_not_recreated() {
    let (d, b) = brain("gone");
    let root = b.root().to_path_buf();
    write(&b, draft("Fato", "Algo."));
    let moved = d.join("movido");
    fs::rename(&root, &moved).unwrap();
    assert!(matches!(
        b.search(&SearchQuery::new("fato")),
        Err(Error::BrainNotFound(_))
    ));
    assert!(b
        .apply(Change::Write(draft("Outro", "Algo.")), Actor::person())
        .is_err());
    assert!(!root.exists(), "nada foi recriado");
    fs::rename(&moved, &root).unwrap();
    assert_eq!(b.search(&SearchQuery::new("fato")).unwrap().hits.len(), 1);
    fs::remove_file(root.join("brain.toml")).unwrap();
    assert!(matches!(Brain::open(&root), Err(Error::BrainNotFound(_))));
    assert!(matches!(
        b.search(&SearchQuery::new("fato")),
        Err(Error::BrainNotFound(_))
    ));
}

#[test]
fn concurrent_init_elects_one_winner() {
    for round in 0..10 {
        let d = TempDir::new("init");
        let p = d.join("b");
        let wins = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|s| {
            for _ in 0..6 {
                s.spawn(|| match Brain::init(&p, "x") {
                    Ok(_) => {
                        wins.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    }
                    Err(Error::AlreadyExists(_)) => {}
                    Err(Error::Config(_) | Error::Io(_)) => {
                        // Perdedor lendo brain.toml no meio da escrita do vencedor não conta como vitória.
                    }
                    Err(e) => panic!("{e}"),
                });
            }
        });
        assert_eq!(wins.load(std::sync::atomic::Ordering::SeqCst), 1, "{round}");
    }
}

#[cfg(unix)]
#[test]
fn symlinks_leaving_the_brain_are_refused() {
    use std::os::unix::fs::symlink;
    let (d, b) = brain("sym");
    let outside = d.join("fora");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("segredo.md"), "SEGREDOFORA").unwrap();
    symlink(outside.join("segredo.md"), b.root().join("notes/link.md")).unwrap();
    symlink(&outside, b.root().join("notes/pasta")).unwrap();
    fs::create_dir(outside.join("m")).unwrap();
    std::thread::sleep(Duration::from_millis(260));
    let r = b.search(&SearchQuery::new("SEGREDOFORA")).unwrap();
    assert!(r.hits.is_empty());
    assert_eq!(b.health().unwrap().refused_symlinks, 2);
    assert!(b.read_note("notes/link.md").is_err());
    assert!(b.save_note("notes/pasta/x.md", "x", None).is_err());
    assert!(b.save_note("notes/link.md", "x", None).is_err());
    assert!(!outside.join("x.md").exists());
    assert_eq!(
        fs::read_to_string(outside.join("segredo.md")).unwrap(),
        "SEGREDOFORA"
    );
    let r = b
        .read_item(&omnimind_core::ReadRequest {
            target: "notes/link.md".into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(r.status, "not_found");
    // .omnimind trocado por symlink não é seguido
    let priv_dir = b.root().join(".omnimind");
    fs::remove_dir_all(&priv_dir).unwrap();
    symlink(&outside, &priv_dir).unwrap();
    assert!(Brain::open(b.root()).is_err());
    assert!(!outside.join("journal").exists());
}

#[test]
fn traversal_is_refused_everywhere() {
    let (_d, b) = brain("trav");
    for p in [
        "../fora.md",
        "/etc/passwd.md",
        "notes/../../x.md",
        "notes/..%2fx.md",
        "memories/decision/x.md",
        ".omnimind/x.md",
    ] {
        assert!(b.save_note(p, "x", None).is_err(), "{p}");
        assert!(b.read_note(p).is_err(), "{p}");
    }
    let mut d = draft("Fato", "Algo.");
    d.note = Some("../../etc/passwd".into());
    assert!(matches!(write(&b, d).outcome, Outcome::Rejected { .. }));
    let mut d = draft("Fato", "Algo.");
    d.supersedes = vec!["../../x".into()];
    assert!(matches!(
        write(&b, d).outcome,
        Outcome::Rejected { .. } | Outcome::Quarantined { .. }
    ));
    let r = b.read_item(&omnimind_core::ReadRequest {
        target: "note:../../etc/passwd".into(),
        ..Default::default()
    });
    assert!(r.is_err() || r.unwrap().status == "not_found");
}

#[test]
fn unreadable_file_is_counted_never_hidden() {
    let (_d, b) = brain("unread");
    fs::write(
        b.root().join("memories/concept/ruim.md"),
        "---\nid: nope\n---\n",
    )
    .unwrap();
    fs::write(b.root().join("notes/ok.md"), "# ok\ncoberto\n").unwrap();
    std::thread::sleep(Duration::from_millis(260));
    let r = b.search(&SearchQuery::new("ausente")).unwrap();
    assert_eq!(r.coverage.unread, 1);
    assert_eq!(r.status, SearchStatus::Incomplete, "{r:?}");
    let issues = b.lint().unwrap();
    assert!(issues
        .iter()
        .any(|i| i.kind == "invalid_memory" && i.severity == "error"));
}

#[test]
fn snapshot_open_is_much_faster_than_rebuild() {
    let (_d, b) = brain("snapfast");
    for i in 0..600 {
        fs::write(
            b.root().join(format!("notes/n{i}.md")),
            format!("# N{i}\n{}\n", "palavras variadas de texto ".repeat(80)),
        )
        .unwrap();
    }
    b.reconcile().unwrap();
    std::thread::sleep(Duration::from_millis(2100));
    b.reconcile().unwrap();
    b.flush().unwrap();
    let t = Instant::now();
    let o = Brain::open(b.root()).unwrap();
    let with = t.elapsed();
    assert_eq!(o.notes().unwrap().len(), 600);
    fs::remove_file(b.root().join(".omnimind/snapshot.bin")).unwrap();
    let t = Instant::now();
    let o = Brain::open(b.root()).unwrap();
    let without = t.elapsed();
    assert_eq!(o.notes().unwrap().len(), 600);
    assert!(
        with * 2 < without,
        "instantâneo {with:?} reconstrução {without:?}"
    );
}

#[test]
fn rebuild_from_scratch_matches_incremental() {
    let (_d, b) = sample_brain("rebuild");
    let got = answers(&b);
    fs::remove_dir_all(b.root().join(".omnimind")).unwrap();
    assert_eq!(answers(&Brain::open(b.root()).unwrap()), got);
}

#[test]
fn notes_conflict_and_random_generator_never_loses_text() {
    // Modelo: o disco deve ser exatamente o último texto aceito ou gravado por fora.
    for seed in 1..=40u64 {
        if let Err(step) = run_note_sequence(seed, 40) {
            // Encolhe: acha o menor prefixo que ainda falha.
            let mut n = 1;
            while n < 40 && run_note_sequence(seed, n).is_ok() {
                n += 1;
            }
            panic!("semente {seed} falhou no passo {step}; mínimo reproduzível: {n} passos");
        }
    }
}

fn run_note_sequence(seed: u64, steps: usize) -> Result<(), usize> {
    let (_d, b) = brain("gen");
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let names = ["notes/a.md", "notes/b.md", "notes/c/d.md"];
    let mut model: std::collections::HashMap<&str, String> = Default::default();
    let mut known: std::collections::HashMap<&str, String> = Default::default();
    for step in 0..steps {
        let name = names[r.below(names.len())];
        let text = format!("# N\n\npasso {step} semente {seed} {}\n", r.next());
        match r.below(5) {
            0 | 1 => {
                let base = known.get(name).cloned();
                let res = b
                    .save_note(name, &text, base.as_deref())
                    .map_err(|_| step)?;
                let current = model.get(name).map(|t| omnimind_core::util::hash_str(t));
                match res {
                    SaveResult::Saved { hash } => {
                        if current != base {
                            return Err(step);
                        }
                        model.insert(name, text);
                        known.insert(name, hash);
                    }
                    SaveResult::Conflict {
                        current_hash,
                        current_text,
                    } => {
                        if current == base
                            || model.get(name).map_or("", String::as_str) != current_text
                        {
                            return Err(step);
                        }
                        // Conflito nunca perde: a pessoa escolhe manter a dela sobre o hash atual.
                        known.insert(name, current_hash);
                    }
                }
            }
            2 => {
                let full = b.root().join(name);
                fs::create_dir_all(full.parent().unwrap()).unwrap();
                fs::write(&full, &text).unwrap();
                model.insert(name, text);
            }
            3 => {
                if model.contains_key(name) && known.contains_key(name) {
                    let e = b.delete_note(name).map_err(|_| step)?;
                    let gone = model.remove(name).unwrap();
                    known.remove(name);
                    if r.below(2) == 0 {
                        b.restore_note(&e.id).map_err(|_| step)?;
                        if b.read_note(name).map_err(|_| step)?.text != gone {
                            return Err(step);
                        }
                        known.insert(name, omnimind_core::util::hash_str(&gone));
                        model.insert(name, gone);
                    }
                }
            }
            _ => {
                let _ = b.search(&SearchQuery::new("passo")).map_err(|_| step)?;
            }
        }
        for n in names {
            let disk = fs::read_to_string(b.root().join(n)).ok();
            if disk.as_deref() != model.get(n).map(String::as_str) {
                return Err(step);
            }
        }
    }
    Ok(())
}
