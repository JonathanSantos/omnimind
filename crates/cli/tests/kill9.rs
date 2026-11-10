//! `kill -9` no meio da escrita, com a costura `OMNIMIND_TEST_PAUSE_MS` entre o temporário e o rename.

mod common;

use common::*;
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;

fn omnimind_tmp_files(brain: &std::path::Path) -> usize {
    std::fs::read_dir(brain.join(".omnimind/tmp")).map_or(0, |d| d.count())
}

#[test]
fn kill_during_write_leaves_no_partial_file() {
    let d = TempDir::new("kill");
    let brain = init_brain(&d);
    let mut killed = 0;
    for i in 0..6 {
        let mut c = cmd(d.path())
            .args([
                "remember",
                "--brain",
                brain.to_str().unwrap(),
                "--key",
                &format!("k{i}"),
                &format!("fato morto {i}"),
            ])
            .env("OMNIMIND_TEST_PAUSE_MS", "2000")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        // Espera o temporário aparecer: só então está no meio da escrita.
        let mut waited = 0;
        while omnimind_tmp_files(&brain) == i && waited < 400 {
            std::thread::sleep(Duration::from_millis(5));
            waited += 1;
        }
        assert!(
            omnimind_tmp_files(&brain) > i,
            "temporário nunca apareceu: costura inútil"
        );
        c.kill().unwrap(); // SIGKILL no PID que este teste criou
        c.wait().unwrap();
        killed += 1;
    }
    assert_eq!(killed, 6);
    // Nada parcial em memories/
    let files: Vec<_> = std::fs::read_dir(brain.join("memories/concept"))
        .unwrap()
        .flatten()
        .collect();
    assert!(files.is_empty(), "{files:?}");
    // Trava morreu com o processo: a próxima escrita passa na hora.
    let o = run(
        d.path(),
        &[
            "remember",
            "--brain",
            brain.to_str().unwrap(),
            "--json",
            "fato vivo",
        ],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: Value = serde_json::from_slice(
        &run(
            d.path(),
            &["doctor", "--brain", brain.to_str().unwrap(), "--json"],
        )
        .stdout,
    )
    .unwrap();
    assert_eq!(v["health"]["memories"], 1);
    assert_eq!(v["health"]["unread"], 0);
}

#[test]
fn kill_between_rename_and_event_is_reconciled() {
    // Disco sem evento: arquivo novo aparece como `external` na reconciliação, uma vez.
    let d = TempDir::new("kill2");
    let brain = init_brain(&d);
    let o = run(
        d.path(),
        &[
            "remember",
            "--brain",
            brain.to_str().unwrap(),
            "--json",
            "fato base do teste",
        ],
    );
    let a: Value = serde_json::from_slice(&o.stdout).unwrap();
    let id = a["id"].as_str().unwrap();
    // simula a janela: arquivo trocado com evento ausente
    let dir = brain.join("memories/concept");
    let f = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .next()
        .unwrap()
        .path();
    let text = std::fs::read_to_string(&f)
        .unwrap()
        .replace("fato base", "fato novo");
    std::fs::write(&f, text).unwrap();
    for _ in 0..2 {
        assert!(
            run(d.path(), &["doctor", "--brain", brain.to_str().unwrap()])
                .status
                .success()
        );
    }
    let o = run(
        d.path(),
        &[
            "log",
            "--brain",
            brain.to_str().unwrap(),
            "--json",
            "--limit",
            "100",
        ],
    );
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    let n = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["ev"] == "external" && e["target"] == id)
        .count();
    assert_eq!(n, 1, "{v}");
    // undo do evento antigo recusa, porque arquivo e diário divergem
    let ev = a["event"].as_str().unwrap();
    assert_eq!(
        run(d.path(), &["undo", "--brain", brain.to_str().unwrap(), ev])
            .status
            .code(),
        Some(1)
    );
}

#[test]
fn kill_while_undoing_a_session_is_resumed() {
    let d = TempDir::new("kill3");
    let brain = init_brain(&d);
    let b = brain.to_str().unwrap();
    for t in ["um", "dois", "três"] {
        let o = run(
            d.path(),
            &[
                "remember",
                "--brain",
                b,
                "--session",
                "estrato",
                &format!("fato de número {t} desta sessão"),
            ],
        );
        assert!(o.status.success());
    }
    let mut c = cmd(d.path())
        .args(["undo", "--brain", b, "--session", "estrato"])
        .env("OMNIMIND_TEST_PAUSE_MS", "1500")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let plans = brain.join(".omnimind/plans");
    let mut waited = 0;
    while waited < 400 && std::fs::read_dir(&plans).map_or(0, |d| d.count()) == 0 {
        std::thread::sleep(Duration::from_millis(5));
        waited += 1;
    }
    std::thread::sleep(Duration::from_millis(1600));
    c.kill().unwrap();
    c.wait().unwrap();
    // próxima trava conclui o plano do processo morto
    let o = run(
        d.path(),
        &["remember", "--brain", b, "fato posterior qualquer"],
    );
    assert!(o.status.success());
    let left = std::fs::read_dir(brain.join("memories/concept"))
        .unwrap()
        .count();
    assert!(left <= 1, "plano incompleto: {left}");
    assert_eq!(std::fs::read_dir(&plans).map_or(0, |d| d.count()), 0);
}
