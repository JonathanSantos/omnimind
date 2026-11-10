//! QA only: drives the real app view and the real data thread against a brain on disk.
//! Usage: qa-notes-real <out-dir> <brain>
use app::harness::Harness;
use app::input::{Key, Mods};
use app::shell::Page;
use std::sync::mpsc::channel;

fn main() {
    let dir = std::env::args().nth(1).unwrap();
    let brain = std::env::args().nth(2).unwrap();
    let (ctx, crx) = channel();
    let (mtx, mrx) = channel();
    let mut h = Harness::new(1440, 900, 2.0, ctx, mrx);
    let _ = app::data::spawn(crx, mtx, Box::new(|| {}), Some(brain), false);
    std::thread::sleep(std::time::Duration::from_millis(1500));
    h.run(10);
    let wait = |h: &mut Harness| {
        for _ in 0..6 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            h.run(10);
        }
    };
    h.page(Page::Notes);
    wait(&mut h);
    h.shot(&format!("{dir}/20-notes-landing.png"));
    h.cmd('k');
    h.run(10);
    h.typ("Rota certa");
    wait(&mut h);
    h.shot(&format!("{dir}/21-palette-search.png"));
    h.key(Key::Enter, Mods::default());
    wait(&mut h);
    wait(&mut h);
    h.shot(&format!("{dir}/22-note-real.png"));
    h.cmd('k');
    h.run(10);
    h.typ("Nome estranho");
    wait(&mut h);
    h.key(Key::Enter, Mods::default());
    wait(&mut h);
    wait(&mut h);
    h.shot(&format!("{dir}/23-note-odd-name.png"));
    h.page(Page::Strata);
    wait(&mut h);
    // First "Liberar" button of the held band, logical coordinates of the 1440x900 window.
    h.click(977.0, 243.0);
    wait(&mut h);
    h.shot(&format!("{dir}/24-after-liberar.png"));
    wait(&mut h);
    wait(&mut h);
    h.shot(&format!("{dir}/25-after-liberar-settled.png"));
}
