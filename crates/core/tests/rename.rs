//! Rename de nota: só os links que apontam para ela, sem sequestro.

mod common;

use common::*;
use omnimind_core::{Brain, Error, SaveResult};

fn put(b: &Brain, p: &str, t: &str) {
    assert!(
        matches!(b.save_note(p, t, None).unwrap(), SaveResult::Saved { .. }),
        "{p}"
    );
}

fn text(b: &Brain, p: &str) -> String {
    b.read_note(p).unwrap().text
}

#[test]
fn updates_only_links_that_point_to_it() {
    let (_d, b) = brain("rn");
    put(&b, "notes/x/Foo.md", "# Foo\n");
    put(&b, "notes/y/Outra.md", "---\ntitle: Foo\n---\ncorpo\n");
    put(
        &b,
        "notes/a.md",
        "[[Foo]] e [[x/Foo]] e [[Outra]] e `[[Foo]]`\n```\n[[Foo]]\n```\n",
    );
    let r = b.rename_note("notes/x/Foo.md", "notes/x/Bar.md").unwrap();
    assert_eq!(r.links_rewritten, 2);
    let a = text(&b, "notes/a.md");
    assert_eq!(
        a,
        "[[Bar]] e [[x/Bar]] e [[Outra]] e `[[Foo]]`\n```\n[[Foo]]\n```\n"
    );
    assert!(b.read_note("notes/x/Foo.md").is_err());
    assert!(b.read_note("notes/x/Bar.md").is_ok());
}

#[test]
fn same_title_elsewhere_is_not_hijacked() {
    let (_d, b) = brain("hijack");
    // Dois arquivos com o mesmo nome: o mais curto ganha a resolução.
    put(&b, "notes/a/Plano.md", "# Plano A\n");
    put(&b, "notes/bb/Plano.md", "# Plano B\n");
    put(&b, "notes/link.md", "[[Plano]] e [[bb/Plano]]\n");
    let r = b
        .rename_note("notes/bb/Plano.md", "notes/bb/Plano Novo.md")
        .unwrap();
    assert_eq!(r.links_rewritten, 1);
    assert_eq!(text(&b, "notes/link.md"), "[[Plano]] e [[bb/Plano Novo]]\n");
}

#[test]
fn rename_that_would_capture_a_link_is_refused() {
    let (_d, b) = brain("capture");
    put(&b, "notes/a.md", "---\ntitle: Destino\n---\ncorpo\n");
    put(&b, "notes/b.md", "# B\n");
    put(&b, "notes/c.md", "[[Destino]]\n");
    match b.rename_note("notes/b.md", "notes/Destino.md") {
        Err(Error::RenameCapturesLinks { notes, .. }) => assert_eq!(notes, vec!["notes/c.md"]),
        other => panic!("{other:?}"),
    }
    assert!(b.read_note("notes/b.md").is_ok(), "nada mudou");
    assert_eq!(text(&b, "notes/c.md"), "[[Destino]]\n");
}

#[test]
fn forms_of_link_survive() {
    let (_d, b) = brain("forms");
    put(&b, "notes/Decisão Final.md", "# x\n");
    put(
        &b,
        "notes/t.md",
        "| a | b |\n|---|---|\n| [[Decisão Final\\|ali]] | [[decisao final#sec]] |\n[[Decisão Final|apelido]] [[ Decisão Final ]] 🍲[[Decisão Final]]漢\n",
    );
    b.rename_note("notes/Decisão Final.md", "notes/Fim.md")
        .unwrap();
    assert_eq!(
        text(&b, "notes/t.md"),
        "| a | b |\n|---|---|\n| [[Fim\\|ali]] | [[Fim#sec]] |\n[[Fim|apelido]] [[ Fim ]] 🍲[[Fim]]漢\n"
    );
}

#[test]
fn nfd_names_and_chains() {
    let (_d, b) = brain("nfd");
    put(&b, "notes/Reuniao.md", "# Reuniao\n");
    put(&b, "notes/n.md", "[[Reunia\u{0303}o]]\n"); // NFD com til, casa sem acento
    b.rename_note("notes/Reuniao.md", "notes/R2.md").unwrap();
    assert_eq!(text(&b, "notes/n.md"), "[[R2]]\n");
    b.rename_note("notes/R2.md", "notes/R3.md").unwrap();
    b.rename_note("notes/R3.md", "notes/sub/R4.md").unwrap();
    assert_eq!(text(&b, "notes/n.md"), "[[R4]]\n");
}

#[test]
fn self_link_and_folder_collision() {
    let (_d, b) = brain("selflink");
    put(&b, "notes/a.md", "# A\nveja [[a]]\n");
    put(&b, "notes/b.md", "# B\n");
    assert!(matches!(
        b.rename_note("notes/a.md", "notes/b.md"),
        Err(Error::AlreadyExists(_))
    ));
    b.rename_note("notes/a.md", "notes/c.md").unwrap();
    assert_eq!(text(&b, "notes/c.md"), "# A\nveja [[c]]\n");
}

#[test]
fn memory_source_follows_nothing_and_goes_stale_after_rename() {
    let (_d, b) = brain("msrc");
    put(&b, "notes/a.md", "# A\nfato\n");
    b.read_item(&omnimind_core::ReadRequest {
        target: "notes/a.md".into(),
        distill: true,
        ..Default::default()
    })
    .unwrap();
    let mut d = draft("Fato A", "Um fato.");
    d.note = Some("notes/a.md".into());
    let w = write(&b, d);
    b.rename_note("notes/a.md", "notes/z.md").unwrap();
    assert_eq!(
        b.memory(&w.id.unwrap()).unwrap().unwrap().meta.state,
        omnimind_core::MemoryState::Stale
    );
}

#[test]
fn old_private_folder_is_migrated_once() {
    let root = std::env::temp_dir().join(format!("omnimind-migrate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let b = omnimind_core::Brain::init(&root, "Migra").expect("init");
    b.save_note("notes/a.md", "# a\n", None).expect("note");
    b.flush().expect("flush");
    drop(b);
    std::fs::rename(root.join(".omnimind"), root.join(".varzea")).unwrap();
    std::fs::write(root.join(".gitignore"), ".varzea/\n").unwrap();

    let b = omnimind_core::Brain::open(&root).expect("open");
    assert!(root.join(".omnimind/journal").is_dir());
    assert!(!root.join(".varzea").exists());
    assert_eq!(std::fs::read_to_string(root.join(".gitignore")).unwrap(), ".omnimind/\n");
    assert_eq!(b.notes().unwrap().len(), 1);
    drop(b);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn concurrent_opens_survive_the_migration() {
    for round in 0..12 {
        let root = std::env::temp_dir().join(format!("omnimind-race-{}-{round}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        drop(omnimind_core::Brain::init(&root, "Corrida").expect("init"));
        std::fs::rename(root.join(".omnimind"), root.join(".varzea")).unwrap();
        let handles: Vec<_> = (0..6)
            .map(|_| {
                let r = root.clone();
                std::thread::spawn(move || omnimind_core::Brain::open(&r).map(|_| ()))
            })
            .collect();
        for h in handles {
            h.join().unwrap().expect("open");
        }
        assert!(root.join(".omnimind").is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }
}
