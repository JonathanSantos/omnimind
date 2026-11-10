//! Leitura de verdade, de ponta a ponta: a IA lê pelo núcleo, o diário registra,
//! a thread de dados percebe e o grafo acende, nas três formas.
use app::command::{Cmd, Msg};
use app::data;
use app::harness::Harness;
use graph::Form;
use omnimind_core::{Actor, Brain, Change, MemoryDraft, MemoryType, ReadRequest};
use std::sync::mpsc::channel;
use std::time::Duration;

fn frames_until(h: &mut Harness, what: &str, mut cond: impl FnMut(&Harness) -> bool) {
    for _ in 0..900 {
        h.frame(1.0 / 60.0);
        if cond(h) {
            return;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
    panic!("not reached: {what}");
}

#[test]
fn what_the_ai_reads_lights_up_in_every_form() {
    let home = std::env::temp_dir().join(format!("omnimind-reads-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);
    let root = home.join("cofre");
    let b = Brain::init(&root, "Leituras").unwrap();
    b.save_note("notes/projetos/trilho.md", "# Trilho\n\nRoteiro de trilhas.\n\n- **Grupo:** Projetos\n- **Pasta:** `~/Developer/projects/trilho`\n", None).unwrap();
    b.save_note("notes/receitas/cuscuz.md", "# Cuscuz\n\nFlocão hidratado por dez minutos, vapor por oito.\n", None).unwrap();
    let mut ids = Vec::new();
    for (title, summary) in [("Mapa sem conexão", "O mapa do trilho abre sem conexão."), ("Trilhas em GPX", "As trilhas entram em GPX."), ("Altimetria por trecho", "A altimetria é calculada por trecho.")] {
        let d = MemoryDraft { mem_type: Some(MemoryType::Decision), title: title.into(), summary: summary.into(), ..Default::default() };
        ids.push(b.apply(Change::Write(d), Actor::agent("claude-code", "s0").with_cwd("/Users/x/Developer/projects/trilho")).unwrap().id.expect("written"));
    }

    let (ctx, crx) = channel::<Cmd>();
    let (mtx, mrx) = channel::<Msg>();
    let join = data::spawn(crx, mtx, Box::new(|| {}), Some(root.to_string_lossy().into_owned()), false);
    let mut h = Harness::new(1200, 800, 1.0, ctx.clone(), mrx);
    h.app.graph.reduce_motion = true;

    // O que a IA lê: duas memórias e uma nota, uma depois da outra, como numa sessão.
    let targets = [ids[0].clone(), "notes/receitas/cuscuz.md".to_string(), ids[2].clone()];
    for form in [Form::Bundles, Form::Roots, Form::Universe] {
        h.app.graph.set_form(form);
        frames_until(&mut h, "graph drawn", |h| h.app.model.memories.len() == 3 && h.app.graph.counts().0 == 5);
        h.settle(900);
        assert_eq!(h.app.graph.reads_shown(), 0, "{form:?}: nothing is lit before the AI reads");
        for (k, target) in targets.iter().enumerate() {
            b.read_item(&ReadRequest { target: target.clone(), ..Default::default() }).unwrap();
            let what = format!("{form:?}: read {k} of {target} reaches the graph");
            frames_until(&mut h, &what, |h| h.app.graph.reads_shown() == k + 1);
        }
        assert!(h.frame(1.0 / 60.0).2, "{form:?}: a read animates");
        if form != Form::Universe {
            assert_eq!(h.app.graph.arcs_shown(), 2, "{form:?}: the light travels from each item to the next");
        }
        let spent = h.settle(2000);
        assert!(spent < 2000, "{form:?}: the reading fades and stops, spent {spent}");
        assert_eq!(h.app.graph.reads_shown(), 0, "{form:?}");
    }

    h.app.flush_all();
    drop(ctx);
    join.join().unwrap();
    let _ = std::fs::remove_dir_all(&home);
}
