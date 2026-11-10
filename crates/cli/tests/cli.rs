//! Contrato dos comandos, conferido no binário.

mod common;

use common::*;
use serde_json::Value;
use std::fs;

fn b(p: &std::path::Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn lint_exit_code_follows_errors() {
    let d = TempDir::new("lint");
    let brain = init_brain(&d);
    assert_eq!(
        run(d.path(), &["lint", "--brain", b(&brain)]).status.code(),
        Some(0)
    );
    fs::write(
        brain.join("memories/concept/ruim.md"),
        "---\nid: nope\n---\n",
    )
    .unwrap();
    let o = run(d.path(), &["lint", "--brain", b(&brain), "--json"]);
    assert_eq!(o.status.code(), Some(1));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["errors"], 1);
}

#[test]
fn human_output_is_portuguese_and_points_to_next_step() {
    let d = TempDir::new("human");
    let p = d.join("novo");
    let o = run(d.path(), &["init", b(&p)]);
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("criado") && out.contains("omnimind connect"),
        "{out}"
    );
    let o = run(
        d.path(),
        &["remember", "--brain", b(&p), "Prefere testes de verdade"],
    );
    assert!(String::from_utf8_lossy(&o.stdout).contains("Gravada."));
    let o = run(d.path(), &["undo", "--brain", b(&p), "NADA"]);
    assert!(String::from_utf8_lossy(&o.stderr).contains("Não encontrado"));
    let o = run(d.path(), &["remember", "--brain", b(&p)]);
    assert!(String::from_utf8_lossy(&o.stderr).starts_with("Uso:"));
}

#[test]
fn doctor_declares_what_it_writes() {
    let d = TempDir::new("doctor");
    let brain = init_brain(&d);
    let o = run(d.path(), &["doctor", "--brain", b(&brain), "--json"]);
    assert!(o.status.success());
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert!(v["writes"].is_string());
    assert_eq!(v["health"]["notes"], 0);
}

#[test]
fn remember_search_read_undo_log() {
    let d = TempDir::new("flow");
    let brain = init_brain(&d);
    let o = run(
        d.path(),
        &[
            "remember",
            "--brain",
            b(&brain),
            "--type",
            "preference",
            "--json",
            "Prefere respostas curtas",
        ],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let a: Value = serde_json::from_slice(&o.stdout).unwrap();
    let id = a["id"].as_str().unwrap().to_string();
    let ev = a["event"].as_str().unwrap().to_string();
    let o = run(
        d.path(),
        &[
            "search",
            "--brain",
            b(&brain),
            "--json",
            "respostas",
            "curtas",
        ],
    );
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["hits"][0]["id"], id.as_str());
    assert_eq!(v["hits"][0]["authority"], "person");
    let o = run(d.path(), &["read", "--brain", b(&brain), &id]);
    assert!(String::from_utf8_lossy(&o.stdout).contains("Prefere respostas"));
    let o = run(d.path(), &["log", "--brain", b(&brain)]);
    assert!(String::from_utf8_lossy(&o.stdout).contains("wrote"));
    assert!(run(d.path(), &["undo", "--brain", b(&brain), &ev])
        .status
        .success());
    let o = run(
        d.path(),
        &["search", "--brain", b(&brain), "--json", "respostas"],
    );
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["status"], "no_match");
    let o = run(
        d.path(),
        &["remember", "--brain", b(&brain), "senha=hunter22 do banco"],
    );
    assert_eq!(o.status.code(), Some(1), "rejeitada sai com erro");
}

#[test]
fn terminal_output_escapes_what_came_from_disk() {
    let d = TempDir::new("term");
    let brain = init_brain(&d);
    fs::write(
        brain.join("notes/x.md"),
        "---\ntitle: \"linha\\nforjada\\u202E\"\n---\ncorpo marcador\n",
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));
    let o = run(d.path(), &["search", "--brain", b(&brain), "marcador"]);
    let s = String::from_utf8_lossy(&o.stdout);
    assert!(!s.contains('\u{202E}'), "{s:?}");
    assert!(!s.contains("linha\nforjada"), "{s:?}");
}

#[test]
fn brain_discovery_and_errors_are_clean() {
    let d = TempDir::new("disc");
    let brain = init_brain(&d);
    let o = cmd(d.path())
        .args(["doctor"])
        .current_dir(&brain)
        .output()
        .unwrap();
    assert!(o.status.success());
    let o = cmd(d.path())
        .args(["doctor", "--json"])
        .current_dir(d.path())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["error"], "brain_not_found");
    assert!(
        !v["message"]
            .as_str()
            .unwrap()
            .contains(d.path().to_str().unwrap()),
        "sem caminho absoluto"
    );
    assert_eq!(run(d.path(), &["inexistente"]).status.code(), Some(2));
}

#[test]
fn sleep_portrait_agenda_export() {
    let d = TempDir::new("misc");
    let brain = init_brain(&d);
    run(
        d.path(),
        &[
            "remember",
            "--brain",
            b(&brain),
            "--type",
            "preference",
            "Gosta de testes de verdade",
        ],
    );
    assert!(run(d.path(), &["sleep", "--brain", b(&brain)])
        .status
        .success());
    let o = run(d.path(), &["portrait", "--brain", b(&brain)]);
    assert!(String::from_utf8_lossy(&o.stdout).contains("Gosta de testes"));
    assert!(run(d.path(), &["agenda", "--brain", b(&brain)])
        .status
        .success());
    let out = d.join("saida");
    assert!(run(
        d.path(),
        &["export", "--brain", b(&brain), "--out", b(&out)]
    )
    .status
    .success());
    assert!(out.join("memories/preference").is_dir());
}

#[test]
fn remember_understands_a_preference_and_keeps_the_title_short() {
    let d = TempDir::new("remember-kind");
    let p = init_brain(&d);
    let o = run(
        d.path(),
        &["remember", "--brain", p.to_str().unwrap(), "--json", "Prefiro borda de 1px a sombra. Sombra fica só no que flutua, como a busca rápida e os menus."],
    );
    assert!(o.status.success());
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("memories/preference/prefiro-borda-de-1px-a-sombra.md"), "{out}");
    let o = run(
        d.path(),
        &["remember", "--brain", p.to_str().unwrap(), "--json", "O deploy da Rota Certa usa duas etapas e a migração roda sempre antes da aplicação, nesta ordem fixa"],
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("memories/preference/"), "sempre vira preferência: {out}");
    assert!(!out.contains("nesta-ordem-fixa"), "{out}");
}

#[test]
fn a_rule_without_a_period_keeps_its_meaning_in_the_title() {
    let d = TempDir::new("remember-rule");
    let p = init_brain(&d);
    let o = run(
        d.path(),
        &["remember", "--brain", p.to_str().unwrap(), "--json", "Interface em Familjen Grotesk, com Sometype Mono só para datas, contagens e atalhos"],
    );
    assert!(o.status.success());
    let file = std::fs::read_dir(p.join("memories/concept")).unwrap().next().unwrap().unwrap();
    let text = std::fs::read_to_string(file.path()).unwrap();
    let title = text.lines().find(|l| l.starts_with("title:")).unwrap();
    assert!(title.contains("contagens e atalhos"), "{title}");
}
