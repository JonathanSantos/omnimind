//! Feixes e Raízes de ponta a ponta, sem janela: zoom, arrasto, luz entre leituras e ligações no foco.
use app::command::{Cmd, Msg};
use app::demo;
use app::harness::Harness;
use app::input::Event;
use app::model::{Live, NodeKind};
use gfx::Rect;
use graph::Form;
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

/// Área do grafo numa janela de 1200 por 800.
const STAGE: Rect = Rect { x: 232.0, y: 56.0, w: 968.0, h: 744.0 };

fn open(form: Form) -> (Harness, Sender<Msg>) {
    let (ctx, _crx) = channel::<Cmd>();
    let (mtx, mrx) = channel::<Msg>();
    let mut h = Harness::new(1200, 800, 1.0, ctx, mrx);
    h.app.graph.reduce_motion = true;
    h.app.graph.form = form;
    mtx.send(Msg::Snapshot(demo::shared())).unwrap();
    for _ in 0..600 {
        h.frame(1.0 / 60.0);
        if h.app.graph.counts().0 > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(h.app.graph.counts().0 > 0, "the graph geometry never arrived");
    h.settle(900);
    (h, mtx)
}

fn memories(h: &Harness) -> Vec<String> {
    h.app.model.graph.nodes.iter().filter(|n| !matches!(n.kind, NodeKind::Note)).map(|n| n.id.clone()).collect()
}

/// Primeiro item que o mouse de fato alcança: outro pode estar por cima.
fn reachable(h: &mut Harness) -> (String, u32, f32, f32) {
    for id in memories(h) {
        let item = h.app.graph.item_of(&id).expect("item");
        let Some((x, y)) = h.app.graph.screen_of(item, STAGE) else { continue };
        if !STAGE.contains(x, y) {
            continue;
        }
        h.app.event(Event::MouseMove(x, y));
        h.run(2);
        if h.app.graph.hover == Some(item) {
            return (id, item, x, y);
        }
    }
    panic!("no item could be reached by the mouse");
}

#[test]
fn zoom_keeps_the_point_under_the_mouse_and_double_click_resets() {
    for form in [Form::Bundles, Form::Roots] {
        let (mut h, _mtx) = open(form);
        let (_, item, x, y) = reachable(&mut h);
        let other = h.app.graph.item_of(memories(&h).last().unwrap()).unwrap();
        let far = h.app.graph.screen_of(other, STAGE).unwrap();
        h.app.event(Event::Scroll(0.0, 300.0));
        h.run(3);
        let (zoom, _) = h.app.graph.view();
        assert!(zoom > 1.5, "{form:?}: scrolling up zooms in, got {zoom}");
        let now = h.app.graph.screen_of(item, STAGE).unwrap();
        assert!((now.0 - x).abs() < 1.0 && (now.1 - y).abs() < 1.0, "{form:?}: the item under the mouse moved from {x},{y} to {now:?}");
        let far_now = h.app.graph.screen_of(other, STAGE).unwrap();
        let (before, after) = ((far.0 - x).hypot(far.1 - y), (far_now.0 - x).hypot(far_now.1 - y));
        assert!((after / before - zoom).abs() < 0.05, "{form:?}: everything else moves away by the zoom");
        assert_eq!(h.app.graph.hover, Some(item), "{form:?}: the mouse still points at the same item");

        for _ in 0..80 {
            h.app.event(Event::Scroll(0.0, 400.0));
            h.run(1);
        }
        assert!(h.app.graph.view().0 <= 8.0, "zoom in is bounded");
        for _ in 0..80 {
            h.app.event(Event::Scroll(0.0, -400.0));
            h.run(1);
        }
        assert_eq!(h.app.graph.view(), (1.0, (0.0, 0.0)), "{form:?}: zooming all the way out puts the drawing back");

        h.app.event(Event::Scroll(0.0, 300.0));
        h.run(2);
        let empty = (STAGE.x + 6.0, STAGE.y + 6.0);
        h.app.event(Event::MouseMove(empty.0, empty.1));
        h.app.event(Event::MouseDown(empty.0, empty.1, 2));
        h.app.event(Event::MouseUp(empty.0, empty.1));
        h.run(2);
        assert_eq!(h.app.graph.view(), (1.0, (0.0, 0.0)), "{form:?}: double click resets");
        assert!(h.settle(600) < 600);
    }
}

#[test]
fn dragging_moves_the_drawing_only_when_zoomed_and_is_not_a_click() {
    let (mut h, _mtx) = open(Form::Bundles);
    let (_, _, x, y) = reachable(&mut h);
    let drag = |h: &mut Harness| {
        h.app.event(Event::MouseDown(x, y, 1));
        h.run(1);
        for k in 1..=10 {
            h.app.event(Event::MouseMove(x + k as f32 * 6.0, y + k as f32 * 3.0));
            h.run(1);
        }
        h.app.event(Event::MouseUp(x + 60.0, y + 30.0));
        h.run(2);
    };
    drag(&mut h);
    assert_eq!(h.app.graph.view(), (1.0, (0.0, 0.0)), "with the whole drawing in view there is nowhere to drag to");
    assert!(h.app.detail.is_none(), "a drag that started on an item must not open it");

    h.app.event(Event::MouseMove(x, y));
    h.app.event(Event::Scroll(0.0, 400.0));
    h.run(2);
    let (_, before) = h.app.graph.view();
    drag(&mut h);
    let (_, after) = h.app.graph.view();
    assert!((after.0 - before.0 - 60.0).abs() < 1.0 && (after.1 - before.1 - 30.0).abs() < 1.0, "pan {before:?} -> {after:?}");
    assert!(h.app.detail.is_none());
}

#[test]
fn clicking_an_item_still_opens_it_after_zoom_and_pan() {
    let (mut h, _mtx) = open(Form::Roots);
    let (id, item, x, y) = reachable(&mut h);
    h.app.event(Event::Scroll(0.0, 260.0));
    h.run(2);
    let (nx, ny) = h.app.graph.screen_of(item, STAGE).unwrap();
    assert!((nx - x).abs() < 1.0 && (ny - y).abs() < 1.0);
    h.click(nx, ny);
    h.run(2);
    assert_eq!(h.app.detail.as_deref(), Some(id.as_str()));
}

#[test]
fn reads_in_sequence_send_a_light_from_one_item_to_the_next() {
    for form in [Form::Bundles, Form::Roots] {
        let (mut h, mtx) = open(form);
        h.app.event(Event::MouseMove(10.0, 400.0));
        h.settle(300);
        let ids: Vec<String> = memories(&h).into_iter().step_by(9).take(3).collect();
        mtx.send(Msg::Live(Live::Read { node: ids[0].clone(), client: "Claude Code".into() })).unwrap();
        assert!(h.frame(1.0 / 60.0).2);
        assert_eq!(h.app.graph.arcs_shown(), 0, "{form:?}: the first read has nowhere to come from");
        h.run(30);
        for id in &ids[1..] {
            mtx.send(Msg::Live(Live::Read { node: id.clone(), client: "Claude Code".into() })).unwrap();
            h.run(30);
        }
        assert_eq!(h.app.graph.arcs_shown(), 2, "{form:?}: each later read travels from the one before");
        let spent = h.settle(1500);
        assert!(spent < 1500, "{form:?}: the reading must fade and stop, spent {spent}");
        assert!(!h.frame(1.0 / 60.0).2, "{form:?}: at rest again");
    }
}

#[test]
fn roots_show_the_links_of_the_item_under_the_mouse() {
    let (mut h, _mtx) = open(Form::Roots);
    let mut degree = std::collections::HashMap::<u32, usize>::new();
    for &(a, b, _) in &h.app.model.graph.edges {
        *degree.entry(a).or_default() += 1;
        *degree.entry(b).or_default() += 1;
    }
    let linked: Vec<String> = h.app.model.graph.nodes.iter().enumerate().filter(|(i, _)| degree.get(&(*i as u32)).copied().unwrap_or(0) >= 2).map(|(_, n)| n.id.clone()).collect();
    for id in linked {
        let item = h.app.graph.item_of(&id).unwrap();
        let Some((x, y)) = h.app.graph.screen_of(item, STAGE) else { continue };
        h.app.event(Event::MouseMove(x, y));
        h.run(2);
        if h.app.graph.hover != Some(item) {
            continue;
        }
        assert!(h.app.graph.arcs_shown() >= 2, "the links of the hovered item must be drawn, got {}", h.app.graph.arcs_shown());
        assert!(h.settle(600) < 600, "the focus animation ends");
        return;
    }
    panic!("no linked item could be reached by the mouse");
}
