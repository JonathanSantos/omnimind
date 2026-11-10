//! Prova 1: nota de 50 mil caracteres, digitação e rolagem, tudo medido sem janela.
//! Mede CPU até o submit, que é a latência real; a espera pela GPU vem em separado.
use app::views::editor::EditorView;
use gfx::{Gpu, Globals, Offscreen, Rect, Renderer, Scene, TextEngine};
use std::time::Instant;

fn note(chars: usize) -> String {
    let par = "A decisão de **manter** o índice como cache veio depois de uma sessão difícil: o *autosave* disparava um indexador por gravação, e a busca levava `10 s`. Veja [[Coerência do índice]] e #arquitetura/indice para o contexto completo da ação.\n\n";
    let mut s = String::from("# Nota enorme de teste\n\n");
    let mut i = 0;
    while s.chars().count() < chars {
        i += 1;
        match i % 7 {
            0 => s.push_str(&format!("## Seção {i}\n\n")),
            1 => s.push_str("- [x] item feito com acentuação\n- item comum\n- outro item\n\n"),
            2 => s.push_str("```rust\nfn main() { println!(\"olá\"); }\n```\n\n"),
            3 => s.push_str("> Citação de alguém sobre a memória que envelhece.\n\n"),
            _ => s.push_str(par),
        }
    }
    s
}

fn main() {
    let (w, h, scale) = (1440u32, 900u32, 2.0f32);
    let gpu = Gpu::headless();
    let mut text = TextEngine::new(app::fonts::FONTS, scale);
    let mut r = Renderer::new(&gpu);
    let target = Offscreen::new(&gpu, w * scale as u32, h * scale as u32);
    let src = note(50_000);
    println!("note: {} chars, {} bytes", src.chars().count(), src.len());
    let t = Instant::now();
    let mut ed = EditorView::new(&src);
    println!("open model: {:?}", t.elapsed());
    let area = Rect::new(0.0, 0.0, w as f32, h as f32);
    let mut scene = Scene::new();
    let g = Globals { size: [w as f32, h as f32], scale, time: 0.0 };
    let frame = |ed: &mut EditorView, scene: &mut Scene, text: &mut TextEngine, r: &mut Renderer, time: f32| -> (f64, f64) {
        let t = Instant::now();
        scene.clear();
        ed.paint(area, scene, text, true, time);
        let paint_ms = t.elapsed().as_secs_f64() * 1e3;
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        r.render(&gpu, &mut enc, &target.view, scene, text, g, app::theme::THEME.bg);
        let sub = gpu.queue.submit([enc.finish()]);
        let cpu = t.elapsed().as_secs_f64() * 1e3;
        gpu.device.poll(gfx::wgpu::PollType::Wait { submission_index: Some(sub), timeout: None }).unwrap();
        let _ = paint_ms;
        (cpu, t.elapsed().as_secs_f64() * 1e3)
    };
    let t = Instant::now();
    let f0 = frame(&mut ed, &mut scene, &mut text, &mut r, 0.0);
    println!("first frame: {:?} (cpu {:.2})", t.elapsed(), f0.0);
    // digitação no meio do documento
    let mid = ed.buf.len_chars() / 2;
    ed.buf.set_caret(mid, false);
    let (mid_line, _) = ed.buf.line_byte(mid);
    frame(&mut ed, &mut scene, &mut text, &mut r, 0.0);
    let _ = mid_line;
    for _ in 0..3 {
        ed.reveal_caret(h as f32);
        frame(&mut ed, &mut scene, &mut text, &mut r, 0.0);
    }
    let typed = "memória ação coração não é sempre tão óbvia ";
    let (mut cpu, mut all) = (vec![], vec![]);
    let mut shaped = 0usize;
    for rep in 0..20 {
        for c in typed.chars() {
            let t = Instant::now();
            text.stat_raster = 0; text.stat_shaped = 0;
            ed.buf.insert(&c.to_string());
            ed.reveal_caret(h as f32);
            let edit = t.elapsed().as_secs_f64() * 1e3;
            let (c1, a1) = frame(&mut ed, &mut scene, &mut text, &mut r, rep as f32);
            if c1 > 4.0 || a1 > 10.0 { println!("  slow #{} cpu {:.2} gpu {:.2} raster {} shaped {}", all.len(), c1, a1-c1, text.stat_raster, text.stat_shaped); }
            shaped += text.stat_shaped as usize;
            cpu.push(edit + c1);
            all.push(edit + a1);
        }
    }
    let pct = |v: &mut Vec<f64>, p: f64| { v.sort_by(|a, b| a.partial_cmp(b).unwrap()); v[((v.len() - 1) as f64 * p) as usize] };
    println!("typing {} keys ({shaped} shapes): cpu p50 {:.2} p99 {:.2} max {:.2} | cpu+gpu p50 {:.2} p99 {:.2} max {:.2} ms", all.len(), pct(&mut cpu, 0.5), pct(&mut cpu, 0.99), cpu[cpu.len()-1], pct(&mut all, 0.5), pct(&mut all, 0.99), all[all.len()-1]);
    let mut sc = vec![];
    ed.scroll = 0.0;
    for i in 0..600 {
        ed.scroll = (i as f32) * 40.0;
        sc.push(frame(&mut ed, &mut scene, &mut text, &mut r, i as f32 / 60.0).1);
    }
    println!("scroll 600 frames: p50 {:.2} p99 {:.2} max {:.2} ms; scene {:?}", pct(&mut sc, 0.5), pct(&mut sc, 0.99), sc[sc.len()-1], scene.stats());
    ed.scroll = 0.0;
    ed.buf.set_caret(40, false);
    frame(&mut ed, &mut scene, &mut text, &mut r, 0.0);
    target.save_png(&gpu, std::path::Path::new("shots/proof-editor.png")).unwrap();
}
