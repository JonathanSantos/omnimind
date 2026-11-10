//! Primeira abertura de ponta a ponta: janela nenhuma, app real, núcleo real, HOME temporário.
use app::command::{Cmd, Msg};
use app::data;
use app::harness::Harness;
use app::input::{Event, Key, Mods};
use app::shell::Page;
use std::sync::mpsc::channel;
use std::time::Duration;

fn wait_for(h: &mut Harness, mut cond: impl FnMut(&Harness) -> bool) {
    for _ in 0..600 {
        h.frame(1.0 / 60.0);
        if cond(h) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("condition not reached");
}

#[test]
fn new_person_creates_a_brain_writes_and_comes_back() {
    let home = std::env::temp_dir().join(format!("omnimind-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);

    let (ctx, crx) = channel::<Cmd>();
    let (mtx, mrx) = channel::<Msg>();
    let join = data::spawn(crx, mtx, Box::new(|| {}), None, false);
    let mut h = Harness::new(1440, 900, 2.0, ctx.clone(), mrx);

    wait_for(&mut h, |h| h.app.page == Page::Welcome);
    h.run(30);
    let b = h.app.start_button;
    assert!(b.w > 0.0, "the welcome screen must lay out the start button");
    h.click(b.x + b.w * 0.5, b.y + b.h * 0.5);
    wait_for(&mut h, |h| h.app.page == Page::Graph);
    assert!(home.join("Omnimind/brain.toml").exists(), "the main button did not create the vault");

    h.key(Key::Char('n'), Mods { cmd: true, ..Default::default() });
    h.run(4);
    for c in "Primeira nota".chars() {
        h.app.event(Event::Text(c.to_string()));
    }
    h.run(4);
    h.key(Key::Enter, Mods::default());
    wait_for(&mut h, |h| h.app.notes.path.is_some());
    h.run(2);
    h.app.event(Event::Text("olá, ação".into()));
    h.app.flush_all();
    drop(ctx);
    join.join().unwrap();

    let note = home.join("Omnimind/notes/primeira-nota.md");
    let text = std::fs::read_to_string(&note).expect("note on disk");
    assert!(text.contains("olá, ação"), "{text}");

    // Segunda abertura lembra do brain, sem passar pelas boas-vindas.
    let (ctx2, crx2) = channel::<Cmd>();
    let (mtx2, mrx2) = channel::<Msg>();
    let join2 = data::spawn(crx2, mtx2, Box::new(|| {}), None, false);
    let mut h2 = Harness::new(1440, 900, 2.0, ctx2.clone(), mrx2);
    wait_for(&mut h2, |h| h.app.model.notes.len() == 1);
    assert_ne!(h2.app.page, Page::Welcome);
    let _ = ctx2.send(Cmd::Shutdown);
    join2.join().unwrap();
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn clicking_the_switch_turns_the_project_off_and_on() {
    let (ctx, crx) = channel::<Cmd>();
    let (mtx, mrx) = channel::<Msg>();
    let mut h = Harness::new(1440, 900, 2.0, ctx, mrx);
    mtx.send(Msg::Snapshot(app::demo::shared())).unwrap();
    wait_for(&mut h, |h| !h.app.model.links.is_empty());
    h.page(Page::Settings);
    h.run(3);
    while crx.try_recv().is_ok() {}
    // Coordenadas da tela de Ajustes: rota-certa, ligado, e cuscuz, desligado.
    h.click(1134.0, 404.0);
    h.run(3);
    let sent: Vec<Cmd> = crx.try_iter().collect();
    assert!(sent.iter().any(|c| matches!(c, Cmd::Disconnect { client, project } if client == "claude-code" && project.ends_with("rota-certa"))), "{sent:?}");
    h.click(1134.0, 318.0);
    h.run(3);
    let sent: Vec<Cmd> = crx.try_iter().collect();
    assert!(sent.iter().any(|c| matches!(c, Cmd::Connect { client, project } if client == "claude-code" && project.ends_with("cuscuz"))), "{sent:?}");
}

#[test]
fn graph_answers_the_mouse_and_goes_quiet() {
    let (ctx, _crx) = channel::<Cmd>();
    let (mtx, mrx) = channel::<Msg>();
    let mut h = Harness::new(1440, 900, 2.0, ctx, mrx);
    let model = app::demo::model();
    mtx.send(Msg::Snapshot(app::demo::shared())).unwrap();
    wait_for(&mut h, |h| h.app.graph.counts().0 > 0);
    assert_eq!(h.app.page, Page::Graph);
    let memory = &model.memories[3];
    let item = h.app.graph.item_of(&memory.id).expect("memory on the graph");
    let stage = gfx::Rect::new(232.0, 56.0, 1208.0, 844.0);
    let (x, y) = h.app.graph.screen_of(item, stage).unwrap();

    h.settle(400);
    h.app.event(Event::MouseMove(x, y));
    h.run(3);
    assert_eq!(h.app.graph.hover, Some(item));
    h.click(x, y);
    h.run(3);
    assert_eq!(h.app.detail.as_deref(), Some(memory.id.as_str()));
    // Selecionado e parado: nada mais a animar.
    h.app.event(Event::MouseMove(500.0, 20.0));
    assert!(h.settle(300) < 300, "the graph must stop drawing when nothing moves");

    h.key(Key::Escape, Mods::default());
    assert_eq!(h.app.detail, None);
    mtx.send(Msg::Live(app::model::Live::Read { node: memory.id.clone(), client: "Claude Code".into() })).unwrap();
    assert!(h.frame(1.0 / 60.0).2, "a live read must animate");
    assert!(h.settle(500) < 500, "a live read must end");

    // Trocar de forma e de filtro nunca aponta para item de outra cena.
    h.app.graph.set_form(graph::Form::Roots);
    h.app.graph.set_filter(app::graphview::Filter::Notes);
    wait_for(&mut h, |h| h.app.graph.item_of(&memory.id).is_none());
    h.app.event(Event::MouseMove(x, y));
    h.run(5);
    assert!(h.settle(500) < 500);
}
