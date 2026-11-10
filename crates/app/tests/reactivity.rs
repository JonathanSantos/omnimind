//! Reatividade: o quadro que trata um clique pinta a tela antes de o clique valer.
//! Sem um quadro seguinte, a tela fica velha até o mouse mexer de novo.
use app::command::{Cmd, Msg};
use app::demo;
use app::harness::Harness;
use app::input::{Event, Key, Mods};
use app::shell::Page;
use std::sync::mpsc::channel;

fn rig() -> (Harness, std::sync::mpsc::Sender<Msg>) {
    let (ctx, _crx) = channel::<Cmd>();
    let (mtx, mrx) = channel::<Msg>();
    let mut h = Harness::new(1200, 800, 1.0, ctx, mrx);
    h.app.graph.reduce_motion = true;
    mtx.send(Msg::Snapshot(demo::shared())).unwrap();
    // A geometria do grafo chega de outra thread: espera até a tela parar de verdade.
    for _ in 0..400 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        if h.settle(600) < 600 && h.app.graph.counts().0 > 0 {
            break;
        }
    }
    assert!(!h.frame(1.0 / 60.0).2, "the screen must be at rest before the test acts");
    (h, mtx)
}

/// Linha da barra lateral: Notas, Memórias, Grafo, Atividade, Métricas, Perfil.
fn nav_row(i: usize) -> (f32, f32) {
    (80.0, 42.0 + 36.0 + 52.0 + i as f32 * 36.0 + 17.0)
}

#[test]
fn click_on_the_sidebar_asks_for_the_frame_that_shows_the_new_page() {
    let (mut h, _mtx) = rig();
    let (x, y) = nav_row(1);
    h.app.event(Event::MouseMove(x, y));
    h.settle(120);
    // Toque de trackpad: aperta e solta sem o ponteiro andar, e nenhum evento vem depois.
    h.app.event(Event::MouseDown(x, y, 1));
    h.app.event(Event::MouseUp(x, y));
    let (_, _, more) = h.frame(1.0 / 60.0);
    assert_eq!(h.app.page, Page::Memories, "the click must change the page");
    assert!(more, "the frame that handled the click painted the old page: another frame is required");
    let spent = h.settle(600);
    assert!(spent < 600, "the screen must come to rest again, spent {spent}");
}

#[test]
fn every_input_gets_a_following_frame_and_then_rests() {
    let (mut h, _mtx) = rig();
    h.app.event(Event::Key(Key::Char('k'), Mods { cmd: true, ..Default::default() }));
    assert!(h.frame(1.0 / 60.0).2, "a key press must be followed by one more frame");
    h.settle(600);
    h.app.event(Event::Key(Key::Escape, Mods::default()));
    assert!(h.frame(1.0 / 60.0).2);
    assert!(h.settle(600) < 600);
    h.app.event(Event::Scroll(0.0, -40.0));
    assert!(h.frame(1.0 / 60.0).2, "a scroll must be followed by one more frame");
    assert!(h.settle(600) < 600);
    assert!(!h.frame(1.0 / 60.0).2, "idle again: no frame without a reason");
}

#[test]
fn message_from_the_data_thread_gets_a_following_frame() {
    let (mut h, mtx) = rig();
    h.page(Page::Activity);
    h.settle(600);
    assert!(!h.frame(1.0 / 60.0).2);
    mtx.send(Msg::Toast("Sessão desfeita.".into())).unwrap();
    assert!(h.frame(1.0 / 60.0).2, "what arrives from the data thread must reach the screen");
}
