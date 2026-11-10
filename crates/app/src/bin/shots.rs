//! Harness de telas: abre o app com dados fictícios, passa por cada tela e salva PNG.
use app::command::Msg;
use app::demo;
use app::harness::{percentiles, Harness};
use app::input::{Event, Key, Mods};
use app::model::Live;
use app::shell::Page;
use app::theme;
use graph::Form;
use std::sync::mpsc::channel;

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "shots".into());
    let (ctx, crx) = channel();
    let (mtx, mrx) = channel();
    let mut h = Harness::new(1440, 900, 2.0, ctx, mrx);
    theme::set_dark(true);
    let brain = std::env::args().nth(2);
    let mtx2 = mtx.clone();
    if let Some(b) = brain.clone() {
        // Cofre real: a mesma thread de dados do app.
        let _ = app::data::spawn(crx, mtx2, Box::new(|| {}), Some(b), false);
        std::thread::sleep(std::time::Duration::from_millis(1500));
    } else {
        mtx.send(Msg::Snapshot(demo::shared())).unwrap();
    }
    h.run(5);
    std::thread::sleep(std::time::Duration::from_millis(200));
    println!("settle {}", h.settle(400));
    h.shot(&format!("{dir}/01-graph-bundles.png"));

    h.app.graph.set_form(Form::Roots);
    h.run(3);
    std::thread::sleep(std::time::Duration::from_millis(120));
    h.run(90);
    h.shot(&format!("{dir}/02-graph-roots-opening.png"));
    println!("settle {}", h.settle(400));
    h.shot(&format!("{dir}/03-graph-roots.png"));

    let m = demo::model();
    // Três leituras em sequência, do que estiver no cofre aberto: memória, nota, memória.
    let seen = h.app.model.clone();
    let mut chain: Vec<String> = seen.memories.iter().step_by(9).take(2).map(|n| n.id.clone()).collect();
    if let Some(n) = seen.notes.get(seen.notes.len() / 3) {
        chain.insert(1, n.path.clone());
    }
    for (k, id) in chain.iter().enumerate() {
        mtx.send(Msg::Live(Live::Read { node: id.clone(), client: "Claude Code".into() })).unwrap();
        h.run(if k + 1 == chain.len() { 30 } else { 60 });
    }
    h.shot(&format!("{dir}/04-graph-reading.png"));
    h.settle(300);

    // Universo: abertura, parado, leitura em percurso e o mesmo céu visto de outro ângulo.
    h.app.graph.set_form(Form::Universe);
    h.run(3);
    std::thread::sleep(std::time::Duration::from_millis(400));
    h.run(60);
    h.shot(&format!("{dir}/23-graph-universe-opening.png"));
    println!("settle {}", h.settle(600));
    h.shot(&format!("{dir}/24-graph-universe.png"));
    let ids: Vec<String> = h.app.model.graph.nodes.iter().filter(|n| !matches!(n.kind, app::model::NodeKind::Note)).step_by(11).take(4).map(|n| n.id.clone()).collect();
    for (k, id) in ids.iter().enumerate() {
        mtx.send(Msg::Live(Live::Read { node: id.clone(), client: "Claude Code".into() })).unwrap();
        h.run(if k + 1 == ids.len() { 44 } else { 66 });
    }
    h.shot(&format!("{dir}/25-graph-universe-reading.png"));
    h.settle(600);
    h.app.event(Event::MouseMove(1000.0, 500.0));
    h.app.event(Event::MouseDown(1000.0, 500.0, 1));
    h.run(1);
    for k in 1..=24 {
        h.app.event(Event::MouseMove(1000.0 - k as f32 * 9.0, 500.0 + k as f32 * 3.0));
        h.run(1);
    }
    h.app.event(Event::MouseUp(784.0, 572.0));
    h.app.event(Event::Scroll(0.0, 160.0));
    h.settle(600);
    h.shot(&format!("{dir}/26-graph-universe-turned.png"));
    theme::set_dark(false);
    h.settle(600);
    h.shot(&format!("{dir}/27-light-graph-universe.png"));
    theme::set_dark(true);
    h.app.graph.set_form(Form::Roots);
    h.run(3);
    std::thread::sleep(std::time::Duration::from_millis(200));
    h.settle(400);

    // Raízes com foco: as ligações do item, que a árvore não desenha, aparecem como arcos.
    let stage = gfx::Rect::new(232.0, 56.0, 1208.0, 844.0);
    let mut degree = std::collections::HashMap::<u32, usize>::new();
    for &(x, y, _) in &h.app.model.graph.edges {
        *degree.entry(x).or_default() += 1;
        *degree.entry(y).or_default() += 1;
    }
    let mut ranked: Vec<(usize, String)> = degree.iter().map(|(n, d)| (*d, h.app.model.graph.nodes[*n as usize].id.clone())).collect();
    ranked.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
    let busiest = ranked.get(ranked.len() / 6).map(|r| r.1.clone());
    if let Some((x, y)) = busiest.as_deref().and_then(|id| h.app.graph.item_of(id)).and_then(|i| h.app.graph.screen_of(i, stage)) {
        h.app.event(Event::MouseMove(x, y));
        h.run(40);
        h.shot(&format!("{dir}/28-graph-roots-focus.png"));
        h.app.event(Event::MouseMove(700.0, 20.0));
        h.settle(200);
    }
    // Feixes: duas leituras seguidas, com a luz a caminho do segundo item.
    h.app.graph.set_form(Form::Bundles);
    h.run(3);
    std::thread::sleep(std::time::Duration::from_millis(200));
    h.settle(400);
    let pair: Vec<String> = h.app.model.graph.nodes.iter().filter(|n| !matches!(n.kind, app::model::NodeKind::Note)).step_by(23).take(2).map(|n| n.id.clone()).collect();
    for (k, id) in pair.iter().enumerate() {
        mtx.send(Msg::Live(Live::Read { node: id.clone(), client: "Claude Code".into() })).unwrap();
        h.run(if k == 0 { 60 } else { 30 });
    }
    h.shot(&format!("{dir}/29-graph-bundles-reading.png"));
    h.settle(600);
    // Zoom: o ponto sob o mouse fica no lugar.
    h.app.event(Event::MouseMove(1150.0, 300.0));
    h.app.event(Event::Scroll(0.0, 420.0));
    h.run(20);
    h.shot(&format!("{dir}/30-graph-bundles-zoom.png"));
    h.app.graph.set_form(Form::Roots);
    h.run(3);
    std::thread::sleep(std::time::Duration::from_millis(200));
    h.settle(400);

    mtx.send(Msg::Note { path: "notes/arquitetura/coerência-do-índice.md".into(), text: demo::note_text(), hash: "x".into() }).unwrap();
    h.run(30);
    h.typ("\nDigitando com acentuação: coração, ação 🙂 日本語.");
    h.app.event(Event::Preedit("´".into()));
    h.run(4);
    h.shot(&format!("{dir}/05-notes.png"));
    h.app.event(Event::Preedit(String::new()));

    h.page(Page::Memories);
    h.run(30);
    h.shot(&format!("{dir}/06-memories.png"));
    h.app.detail = Some(m.memories[7].id.clone());
    let long = "Texto longo para conferir a rolagem do painel. ".repeat(120);
    mtx.send(Msg::MemoryBody { id: m.memories[7].id.clone(), hash: m.memories[7].hash.clone(), body: long }).unwrap();
    h.run(40);
    h.shot(&format!("{dir}/07-memory-detail.png"));
    h.app.detail = None;
    h.page(Page::Activity);
    h.run(30);
    h.shot(&format!("{dir}/08-activity.png"));
    h.page(Page::Metrics);
    h.run(50);
    h.shot(&format!("{dir}/09-metrics.png"));
    h.app.event(Event::MouseMove(900.0, 500.0));
    h.app.event(Event::Scroll(0.0, -640.0));
    h.run(30);
    h.shot(&format!("{dir}/10-metrics-lists.png"));
    h.app.event(Event::Scroll(0.0, -900.0));
    h.run(30);
    h.shot(&format!("{dir}/11-metrics-rhythm.png"));
    h.page(Page::Profile);
    h.run(30);
    h.shot(&format!("{dir}/12-profile.png"));
    h.page(Page::Settings);
    h.run(30);
    h.shot(&format!("{dir}/13-settings.png"));
    h.page(Page::Graph);
    h.run(5);
    h.cmd('k');
    h.run(20);
    h.shot(&format!("{dir}/14-palette.png"));
    h.key(Key::Escape, Mods::default());
    h.run(10);

    // Claro: o mesmo app, com o outro conjunto de tokens.
    theme::set_dark(false);
    h.app.graph.set_form(Form::Bundles);
    h.run(3);
    std::thread::sleep(std::time::Duration::from_millis(120));
    h.settle(400);
    h.shot(&format!("{dir}/15-light-graph.png"));
    h.page(Page::Metrics);
    h.run(40);
    h.shot(&format!("{dir}/16-light-metrics.png"));
    h.page(Page::Notes);
    h.run(30);
    h.shot(&format!("{dir}/17-light-notes.png"));
    theme::set_dark(true);
    h.page(Page::Graph);
    h.run(3);
    h.settle(400);

    // Medição com leituras chegando sem parar.
    let (mut cpu, mut all) = (vec![], vec![]);
    for i in 0..240 {
        if i % 20 == 0 {
            let n = &m.memories[i % m.memories.len()];
            mtx.send(Msg::Live(Live::Read { node: n.id.clone(), client: "Claude Code".into() })).unwrap();
        }
        let (c, a, _) = h.frame(1.0 / 60.0);
        cpu.push(c);
        all.push(a);
    }
    let (c50, c99, cm) = percentiles(&mut cpu);
    let (a50, a99, am) = percentiles(&mut all);
    // Estados vazios e erro: também fazem parte do produto.
    let mut empty = app::model::Model { name: "Novo".into(), path: "/Users/voce/Omnimind".into(), generation: 99, ..Default::default() };
    empty.notes = vec![];
    mtx.send(Msg::Snapshot(std::sync::Arc::new(empty))).unwrap();
    h.run(30);
    h.shot(&format!("{dir}/18-empty-graph.png"));
    h.page(Page::Notes);
    h.run(30);
    h.shot(&format!("{dir}/19-empty-notes.png"));
    h.page(Page::Activity);
    h.run(30);
    h.shot(&format!("{dir}/20-empty-activity.png"));
    h.page(Page::Metrics);
    h.run(30);
    h.shot(&format!("{dir}/21-empty-metrics.png"));
    h.page(Page::Graph);
    mtx.send(Msg::NoBrain).unwrap();
    h.run(3);
    std::thread::sleep(std::time::Duration::from_millis(120));
    h.run(150);
    h.shot(&format!("{dir}/22-welcome.png"));
    println!("graph frames: cpu p50 {c50:.2} p99 {c99:.2} max {cm:.2} | total p50 {a50:.2} p99 {a99:.2} max {am:.2}; scene {:?}", h.app.scene.stats());
}
