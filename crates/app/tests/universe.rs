//! Universo de ponta a ponta, sem janela: abre, descansa, gira com o mouse, acende com a leitura e abre a memória clicada.
use app::command::{Cmd, Msg};
use app::demo;
use app::harness::Harness;
use app::input::Event;
use app::model::{Live, NodeKind};
use gfx::Rect;
use graph::Form;
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

/// Área do grafo numa janela de 1200 por 800: tudo menos a barra lateral e a faixa do título.
const STAGE: Rect = Rect { x: 232.0, y: 56.0, w: 968.0, h: 744.0 };

fn open() -> (Harness, Sender<Msg>) {
    let (ctx, _crx) = channel::<Cmd>();
    let (mtx, mrx) = channel::<Msg>();
    let mut h = Harness::new(1200, 800, 1.0, ctx, mrx);
    mtx.send(Msg::Snapshot(demo::shared())).unwrap();
    h.run(3);
    h.app.graph.set_form(Form::Universe);
    for _ in 0..600 {
        h.frame(1.0 / 60.0);
        if h.app.graph.universe_camera().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(h.app.graph.universe_camera().is_some(), "the universe geometry never arrived");
    (h, mtx)
}

#[test]
fn opens_animated_and_then_spends_no_frame() {
    let (mut h, _mtx) = open();
    assert!(h.frame(1.0 / 60.0).2, "arriving plays the opening");
    let spent = h.settle(900);
    assert!(spent < 900, "the opening must end, spent {spent}");
    assert!(!h.frame(1.0 / 60.0).2, "at rest the universe asks for no frame");
    let (items, links) = h.app.graph.counts();
    assert!(items > 100 && links > 0, "{items} items, {links} links");
}

#[test]
fn dragging_turns_the_camera_and_scrolling_zooms() {
    let (mut h, _mtx) = open();
    h.settle(900);
    let before = h.app.graph.universe_camera().unwrap();
    let (x, y) = (STAGE.x + STAGE.w * 0.5, STAGE.y + 40.0);
    h.app.event(Event::MouseMove(x, y));
    h.app.event(Event::MouseDown(x, y, 1));
    h.run(1);
    for k in 1..=20 {
        h.app.event(Event::MouseMove(x + k as f32 * 8.0, y + k as f32 * 2.0));
        h.run(1);
    }
    h.app.event(Event::MouseUp(x + 160.0, y + 40.0));
    h.run(1);
    let turned = h.app.graph.universe_camera().unwrap();
    assert!((turned.yaw - before.yaw).abs() > 0.5, "yaw {} -> {}", before.yaw, turned.yaw);
    assert!(turned.pitch > before.pitch, "dragging down looks from above");
    assert!(h.app.detail.is_none(), "a drag is not a click");
    assert!(h.settle(900) < 900, "the spin after the drag must die out");

    h.app.event(Event::Scroll(0.0, 200.0));
    h.run(2);
    let near = h.app.graph.universe_camera().unwrap();
    assert!(near.dist < turned.dist, "scrolling up comes closer: {} -> {}", turned.dist, near.dist);
    for _ in 0..200 {
        h.app.event(Event::Scroll(0.0, -400.0));
        h.run(1);
    }
    let far = h.app.graph.universe_camera().unwrap();
    assert!(far.dist.is_finite() && far.dist < before.dist * 8.0, "zoom out is bounded: {}", far.dist);
}

#[test]
fn reading_lights_a_path_and_comes_to_rest() {
    let (mut h, mtx) = open();
    h.settle(900);
    let ids: Vec<String> = h.app.model.graph.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Note)).step_by(7).take(3).map(|n| n.id.clone()).collect();
    for id in &ids {
        mtx.send(Msg::Live(Live::Read { node: id.clone(), client: "Claude Code".into() })).unwrap();
        assert!(h.frame(1.0 / 60.0).2, "a read must animate");
        h.run(40);
    }
    let spent = h.settle(1500);
    assert!(spent < 1500, "the reading must fade and stop, spent {spent}");
    assert!(!h.frame(1.0 / 60.0).2);
}

#[test]
fn clicking_a_star_opens_its_memory() {
    let (mut h, _mtx) = open();
    h.settle(900);
    let memories: Vec<String> = h.app.model.graph.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Note)).map(|n| n.id.clone()).collect();
    for id in memories {
        let item = h.app.graph.item_of(&id).expect("item");
        let Some((x, y)) = h.app.graph.screen_of(item, STAGE) else { continue };
        if !STAGE.contains(x, y) {
            continue;
        }
        h.app.event(Event::MouseMove(x, y));
        h.run(2);
        // Outro corpo pode estar na frente deste: vale o primeiro que o mouse realmente alcança.
        if h.app.graph.hover != Some(item) {
            continue;
        }
        h.click(x, y);
        h.run(2);
        assert_eq!(h.app.detail.as_deref(), Some(id.as_str()), "the click must open the memory under the mouse");
        return;
    }
    panic!("no star could be reached by the mouse");
}
