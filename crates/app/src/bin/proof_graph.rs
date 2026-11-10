//! Prova do grafo: Feixes, Raízes e Universo em vários volumes, com leituras chegando, medidos sem janela.
//! `cargo run --release -p app --bin proof-graph`
use app::command::Msg;
use app::harness::{percentiles, Harness};
use app::model::{Graph, GraphNode, Live, MemKind, Model, NodeKind};
use app::shell::Page;
use graph::{rng, Form};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn model(n: usize, links: usize) -> Model {
    let mut r = rng(0x2545F4914F6CDD1D);
    let regions: Vec<String> = ["Rota Certa", "Arquitetura", "Pessoal", "Clientes", "Leituras", "Saúde", "Marca"].map(String::from).to_vec();
    let nodes = (0..n).map(|i| GraphNode { id: format!("n{i}"), kind: if i % 4 == 0 { NodeKind::Note } else { NodeKind::Memory(MemKind::Decision) }, title: format!("Item {i}"), weight: (r() * r() * 9.0).floor(), region: (r() * r() * 7.0) as u32 }).collect();
    let edges = (0..links).map(|_| ((r() * n as f32) as u32, (r() * r() * n as f32) as u32, 1.0)).filter(|e| e.0 != e.1).collect();
    Model { graph: Graph { nodes, edges, regions }, generation: n as u64, ..Default::default() }
}

fn main() {
    println!("{:<9} {:>7} {:>8} {:>9} {:>9} {:>9}", "form", "items", "links", "build ms", "p50 ms", "p99 ms");
    for (n, links) in [(540, 1600), (2000, 6000), (4000, 12000), (10000, 30000)] {
        for (form, name) in [(Form::Bundles, "feixes"), (Form::Roots, "raizes"), (Form::Universe, "universo")] {
            let (ctx, _crx) = channel();
            let (mtx, mrx) = channel();
            let mut h = Harness::new(1440, 900, 2.0, ctx, mrx);
            h.app.graph.form = form;
            mtx.send(Msg::Snapshot(Arc::new(model(n, links)))).unwrap();
            h.page(Page::Graph);
            let t = Instant::now();
            while h.app.graph.counts().0 == 0 && t.elapsed() < Duration::from_secs(30) {
                h.frame(1.0 / 60.0);
                std::thread::sleep(Duration::from_millis(1));
            }
            let build = t.elapsed().as_secs_f64() * 1e3;
            let mut all = vec![];
            for i in 0..300usize {
                if i % 15 == 0 {
                    mtx.send(Msg::Live(Live::Read { node: format!("n{}", (i * 37) % n), client: "Claude Code".into() })).unwrap();
                }
                all.push(h.frame(1.0 / 60.0).1);
            }
            // Os primeiros quadros criam pipelines e sobem a geometria: isso é abertura, não ritmo.
            let (p50, p99, _) = percentiles(&mut all[30..]);
            println!("{name:<9} {n:>7} {links:>8} {build:>9.0} {p50:>9.2} {p99:>9.2}");
        }
    }
}
