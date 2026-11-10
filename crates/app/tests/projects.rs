//! Projetos de ponta a ponta: núcleo real, thread de dados real, app sem janela.
//! Registrar um projeto com nota, mover memórias e ver o grafo se reagrupar.
use app::command::{Cmd, Msg};
use app::data;
use app::graphview::ColorBy;
use app::harness::Harness;
use app::input::{Event, Key, Mods};
use app::model::Model;
use app::views::palette::{Choice, ChoiceKind, ChoiceOption};
use omnimind_core::{Actor, Brain, Change, MemoryDraft, MemoryType};
use std::sync::mpsc::channel;
use std::time::Duration;

fn wait_for(h: &mut Harness, what: &str, mut cond: impl FnMut(&Model) -> bool) {
    for _ in 0..800 {
        h.frame(1.0 / 60.0);
        if cond(&h.app.model) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("not reached: {what}");
}

fn write(b: &Brain, cwd: &str, kind: MemoryType, title: &str, summary: &str) -> String {
    let d = MemoryDraft { mem_type: Some(kind), title: title.into(), summary: summary.into(), ..Default::default() };
    b.apply(Change::Write(d), Actor::agent("claude-code", "s1").with_cwd(cwd)).unwrap().id.expect("written")
}

#[test]
fn a_project_gets_its_note_its_group_and_its_memories() {
    let home = std::env::temp_dir().join(format!("omnimind-projects-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    // Sem isto o teste leria a ligação global de quem o roda.
    std::env::set_var("HOME", &home);
    let root = home.join("cofre");
    let b = Brain::init(&root, "Projetos").unwrap();
    b.save_note("notes/projetos/trilho.md", "# trilho\n\nRoteiro de trilhas.\n\n- **Grupo:** Projetos\n- **Pasta:** `~/Developer/projects/trilho`\n", None).unwrap();
    let screen = write(&b, "/Users/x/Developer/projects/peneira", MemoryType::Decision, "Peneira por lote", "A peneira processa por lote, não por item.");
    write(&b, "/Users/x/Developer/projects/peneira", MemoryType::Decision, "Fila em disco", "A fila da peneira fica em disco.");
    let loose = write(&b, "/Users/x/Developer/projects/scratch-9", MemoryType::Decision, "Gerador em processo separado", "O gerador roda num processo separado.");
    write(&b, "/Users/x/Developer/projects/trilho", MemoryType::Decision, "Mapa sem conexão", "O mapa do trilho abre sem conexão.");
    drop(b);

    let (ctx, crx) = channel::<Cmd>();
    let (mtx, mrx) = channel::<Msg>();
    let join = data::spawn(crx, mtx, Box::new(|| {}), Some(root.to_string_lossy().into_owned()), false);
    let mut h = Harness::new(1200, 800, 1.0, ctx.clone(), mrx);
    h.app.graph.reduce_motion = true;
    wait_for(&mut h, "vault loaded", |m| m.memories.len() == 4);

    // O que o cofre diz antes de qualquer decisão da pessoa.
    let m = h.app.model.clone();
    let info = |m: &Model, slug: &str| m.projects.iter().find(|p| p.slug == slug).cloned().unwrap_or_else(|| panic!("{slug}: {:?}", m.projects));
    assert_eq!((info(&m, "trilho").note.is_some(), info(&m, "trilho").memories), (true, 1));
    assert_eq!((info(&m, "peneira").note.is_some(), info(&m, "peneira").memories, info(&m, "peneira").ephemeral), (false, 2, false));
    assert!(info(&m, "scratch-9").ephemeral);
    assert!(m.global_links.is_empty(), "a pasta pessoal do teste não tem ligação global");
    let node = |m: &Model, id: &str| m.graph.nodes.iter().find(|n| n.id == id).cloned().unwrap_or_else(|| panic!("node {id}"));
    let group = |m: &Model, id: &str| m.graph.regions[node(m, id).region as usize].clone();
    let project = |m: &Model, id: &str| m.graph.projects.get(node(m, id).project as usize).cloned();
    assert_eq!(group(&m, "notes/projetos/trilho.md"), "Projetos");
    assert!(node(&m, "notes/projetos/trilho.md").hub, "a nota do projeto é o centro dele");
    assert_eq!(group(&m, &screen), data::UNREGISTERED_GROUP);
    assert_eq!(project(&m, &screen).as_deref(), Some("Peneira"));

    // Cor por grupo ou por projeto: os mesmos itens, fatias diferentes.
    wait_for(&mut h, "graph drawn", |_| true);
    for _ in 0..400 {
        h.frame(1.0 / 60.0);
        if h.app.graph.counts().0 > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(h.app.graph.theme_names.contains(&"Projetos".to_string()), "{:?}", h.app.graph.theme_names);
    h.app.graph.set_color_by(ColorBy::Project);
    for _ in 0..400 {
        h.frame(1.0 / 60.0);
        std::thread::sleep(Duration::from_millis(2));
        if h.app.graph.theme_names.contains(&"Peneira".to_string()) {
            break;
        }
    }
    assert!(h.app.graph.theme_names.contains(&"Peneira".to_string()), "{:?}", h.app.graph.theme_names);
    assert!(h.app.graph.theme_names.contains(&"trilho".to_string()), "{:?}", h.app.graph.theme_names);

    // A pessoa registra o projeto: escolhe o grupo na paleta, digitando um grupo que ainda não existe.
    h.app.palette.choose(Choice { kind: ChoiceKind::ProjectGroup { slug: "peneira".into() }, label: "Grupo do projeto", options: vec![ChoiceOption { label: "Projetos".into(), detail: "1 projeto".into(), value: Some("Projetos".into()) }], free_text: true });
    h.typ("Jogos");
    h.run(2);
    h.key(Key::Enter, Mods::default());
    wait_for(&mut h, "project note created", |m| m.projects.iter().any(|p| p.slug == "peneira" && p.note.is_some()));
    let m = h.app.model.clone();
    assert_eq!(info(&m, "peneira").group, "Jogos");
    assert_eq!(info(&m, "peneira").note.as_deref(), Some("notes/projetos/peneira.md"));
    assert_eq!(group(&m, &screen), "Jogos", "as memórias seguem a nota para o grupo dela");
    let (a, hub) = (m.graph.nodes.iter().position(|n| n.id == screen).unwrap() as u32, m.graph.nodes.iter().position(|n| n.id == "notes/projetos/peneira.md").unwrap() as u32);
    assert!(m.graph.edges.iter().any(|e| (e.0, e.1) == (a, hub)), "cada memória fica presa à nota do seu projeto");
    assert!(std::fs::read_to_string(root.join("notes/projetos/peneira.md")).unwrap().contains("**Grupo:** Jogos"));

    // O que sobrou na pasta descartável vai, de uma vez, para onde pertence.
    ctx.send(Cmd::MoveProject { from: "scratch-9".into(), to: Some("peneira".into()) }).unwrap();
    wait_for(&mut h, "scratch emptied", |m| m.memories.iter().find(|x| x.id == loose).is_some_and(|x| x.project.as_deref() == Some("peneira")));
    assert!(!h.app.model.projects.iter().any(|p| p.slug == "scratch-9"), "sem memórias, o projeto provisório some");

    // E uma memória vira geral pela paleta, com a primeira opção.
    let mem = h.app.model.memories.iter().find(|x| x.id == loose).cloned().unwrap();
    h.app.palette.choose(Choice { kind: ChoiceKind::MemoryProject { id: mem.id.clone(), hash: mem.hash.clone() }, label: "Projeto desta memória", options: vec![ChoiceOption { label: "Geral".into(), detail: "vale em todo projeto".into(), value: None }, ChoiceOption { label: "Peneira".into(), detail: "Jogos".into(), value: Some("peneira".into()) }], free_text: false });
    h.run(2);
    h.app.event(Event::Key(Key::Enter, Mods::default()));
    wait_for(&mut h, "memory made general", |m| m.memories.iter().find(|x| x.id == loose).is_some_and(|x| x.project.is_none()));
    assert!(!h.app.palette.open, "escolher fecha a paleta");

    h.app.flush_all();
    drop(ctx);
    join.join().unwrap();
    let _ = std::fs::remove_dir_all(&home);
}
