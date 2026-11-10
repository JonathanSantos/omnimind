//! `omnimind connect`: diff, backup, idempotência, e nada fora do projeto sem `--global`.

mod common;

use common::*;
use serde_json::{json, Value};
use std::fs;

fn setup() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let d = TempDir::new("connect");
    let brain = init_brain(&d);
    let proj = d.join("projeto");
    fs::create_dir(&proj).unwrap();
    (d, brain, proj)
}

fn connect(
    d: &TempDir,
    brain: &std::path::Path,
    proj: &std::path::Path,
    target: &str,
    extra: &[&str],
) -> std::process::Output {
    let mut a = vec![
        "connect",
        target,
        "--project",
        proj.to_str().unwrap(),
        "--brain",
        brain.to_str().unwrap(),
    ];
    a.extend_from_slice(extra);
    run(d.path(), &a)
}

fn read(p: std::path::PathBuf) -> Value {
    serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap()
}

#[test]
fn print_writes_nothing() {
    let (d, brain, proj) = setup();
    let o = connect(&d, &brain, &proj, "claude-code", &["--print"]);
    assert!(o.status.success());
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("+++") && out.contains("mudariam"));
    assert_eq!(fs::read_dir(&proj).unwrap().count(), 0);
}

#[test]
fn claude_code_files_are_correct_and_idempotent() {
    let (d, brain, proj) = setup();
    assert!(connect(&d, &brain, &proj, "claude-code", &[])
        .status
        .success());
    let mcp = read(proj.join(".mcp.json"));
    let s = &mcp["mcpServers"]["omnimind"];
    assert!(s.get("type").is_none());
    assert_eq!(s["args"][0], "mcp");
    assert_eq!(s["args"][1], "--brain");
    assert_eq!(s["args"][4], "claude-code");
    let settings = read(proj.join(".claude/settings.json"));
    for ev in [
        "SessionStart",
        "UserPromptSubmit",
        "PostToolUse",
        "PreCompact",
        "Stop",
        "SessionEnd",
    ] {
        let cmd = settings["hooks"][ev][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(cmd.contains(" hook ") && !cmd.contains(".sh"), "{cmd}");
    }
    assert!(proj.join(".claude/skills/omnimind/SKILL.md").exists());
    let sonhar = fs::read_to_string(proj.join(".claude/skills/sonhar/SKILL.md")).unwrap();
    assert!(sonhar.contains("memory_agenda"));
    let o = connect(&d, &brain, &proj, "claude-code", &[]);
    assert!(String::from_utf8_lossy(&o.stdout).contains("0 arquivos alterados"));
    assert!(fs::read_dir(&proj)
        .unwrap()
        .flatten()
        .all(|e| !e.file_name().to_string_lossy().contains("backup")));
}

#[test]
fn existing_config_is_merged_with_backup() {
    let (d, brain, proj) = setup();
    fs::write(
        proj.join(".mcp.json"),
        json!({"mcpServers": {"outro": {"command": "x"}}}).to_string(),
    )
    .unwrap();
    fs::create_dir(proj.join(".claude")).unwrap();
    let mine = json!({"permissions": {"allow": ["Bash(ls)"]}, "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "meu-hook"}]}]}});
    fs::write(proj.join(".claude/settings.json"), mine.to_string()).unwrap();
    assert!(connect(&d, &brain, &proj, "claude-code", &[])
        .status
        .success());
    let mcp = read(proj.join(".mcp.json"));
    assert_eq!(mcp["mcpServers"]["outro"]["command"], "x");
    assert!(mcp["mcpServers"]["omnimind"].is_object());
    let s = read(proj.join(".claude/settings.json"));
    assert_eq!(s["permissions"]["allow"][0], "Bash(ls)");
    let stop = s["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 2);
    assert_eq!(stop[0]["hooks"][0]["command"], "meu-hook");
    let backups: Vec<_> = fs::read_dir(proj.join(".claude"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains("backup"))
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(read(backups[0].path()), mine);
    // rodar de novo não duplica hook
    connect(&d, &brain, &proj, "claude-code", &[]);
    let s = read(proj.join(".claude/settings.json"));
    assert_eq!(s["hooks"]["Stop"].as_array().unwrap().len(), 2);
}

#[test]
fn malformed_config_is_left_untouched() {
    let (d, brain, proj) = setup();
    fs::write(proj.join(".mcp.json"), "{ nao e json").unwrap();
    let o = connect(&d, &brain, &proj, "claude-code", &[]);
    assert!(!o.status.success());
    assert_eq!(
        fs::read_to_string(proj.join(".mcp.json")).unwrap(),
        "{ nao e json"
    );
    assert!(!proj.join(".claude").exists());
}

#[test]
fn commented_config_gets_a_clear_message() {
    let (d, brain, proj) = setup();
    fs::create_dir(proj.join(".vscode")).unwrap();
    let text = "// meus servidores\n{\"servers\": {}}\n";
    fs::write(proj.join(".vscode/mcp.json"), text).unwrap();
    let o = connect(&d, &brain, &proj, "copilot", &[]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("comentários"));
    assert_eq!(
        fs::read_to_string(proj.join(".vscode/mcp.json")).unwrap(),
        text
    );
}

#[test]
fn home_is_untouched_without_global() {
    let (d, brain, proj) = setup();
    let home_claude = d.join(".claude");
    fs::create_dir(&home_claude).unwrap();
    fs::write(home_claude.join("settings.json"), "{}").unwrap();
    assert!(connect(&d, &brain, &proj, "claude-code", &[])
        .status
        .success());
    assert_eq!(
        fs::read_to_string(home_claude.join("settings.json")).unwrap(),
        "{}"
    );
    assert!(!d.join(".claude.json").exists());
    assert_eq!(fs::read_dir(&home_claude).unwrap().count(), 1);
    // `.claude` do projeto apontando para a home não fura a regra
    #[cfg(unix)]
    {
        let p2 = d.join("p2");
        fs::create_dir(&p2).unwrap();
        std::os::unix::fs::symlink(&home_claude, p2.join(".claude")).unwrap();
        let o = connect(&d, &brain, &p2, "claude-code", &[]);
        assert!(!o.status.success());
        assert_eq!(
            fs::read_to_string(home_claude.join("settings.json")).unwrap(),
            "{}"
        );
    }
    // com --global, e HOME temporário, vai para a home
    assert!(connect(&d, &brain, &proj, "claude-code", &["--global"])
        .status
        .success());
    assert!(fs::read_to_string(home_claude.join("settings.json"))
        .unwrap()
        .contains("hook"));
    assert!(d.join(".claude.json").exists());
}

#[test]
fn copilot_uses_its_own_formats() {
    let (d, brain, proj) = setup();
    assert!(connect(&d, &brain, &proj, "copilot", &[]).status.success());
    assert_eq!(
        read(proj.join(".vscode/mcp.json"))["servers"]["omnimind"]["type"],
        "stdio"
    );
    assert_eq!(
        read(proj.join(".github/mcp.json"))["mcpServers"]["omnimind"]["type"],
        "local"
    );
    let h = read(proj.join(".github/hooks/omnimind.json"));
    assert_eq!(h["version"], 1);
    assert!(h["hooks"]["userPromptSubmitted"][0]["bash"]
        .as_str()
        .unwrap()
        .contains("--client copilot"));
    assert!(h["hooks"]["sessionStart"].is_array() && h["hooks"]["agentStop"].is_array());
    assert!(proj.join(".github/skills/sonhar/SKILL.md").exists());
    assert!(!proj.join(".mcp.json").exists());
}

#[test]
fn generated_hook_command_really_runs() {
    let (d, brain, proj) = setup();
    connect(&d, &brain, &proj, "claude-code", &[]);
    let s = read(proj.join(".claude/settings.json"));
    let c = s["hooks"]["PreCompact"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    let o = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(c)
        .env("HOME", d.path())
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0));
}

#[test]
fn connection_from_the_old_name_is_replaced() {
    let (d, brain, proj) = setup();
    let old_hook = format!("/antigo/varzea hook session-start --brain {}", brain.display());
    fs::write(proj.join(".mcp.json"), serde_json::to_string(&json!({"mcpServers": {"varzea": {"command": "/antigo/varzea"}, "outro": {"command": "x"}}})).unwrap()).unwrap();
    fs::create_dir_all(proj.join(".claude")).unwrap();
    fs::write(proj.join(".claude/settings.json"), serde_json::to_string(&json!({"hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": old_hook}]}]}})).unwrap()).unwrap();
    assert!(connect(&d, &brain, &proj, "claude-code", &[]).status.success());
    let mcp = read(proj.join(".mcp.json"));
    assert!(mcp["mcpServers"].get("varzea").is_none());
    assert!(mcp["mcpServers"].get("omnimind").is_some());
    assert!(mcp["mcpServers"].get("outro").is_some(), "o que é de outros fica");
    let settings = fs::read_to_string(proj.join(".claude/settings.json")).unwrap();
    assert!(!settings.contains("varzea"), "{settings}");
}

#[test]
fn copilot_leftovers_from_the_old_name_are_removed() {
    let (d, brain, proj) = setup();
    fs::create_dir_all(proj.join(".vscode")).unwrap();
    fs::create_dir_all(proj.join(".github/hooks")).unwrap();
    fs::create_dir_all(proj.join(".github/skills/varzea")).unwrap();
    fs::write(proj.join(".vscode/mcp.json"), r#"{"servers":{"varzea":{"command":"/antigo/varzea"}}}"#).unwrap();
    fs::write(proj.join(".github/mcp.json"), r#"{"mcpServers":{"varzea":{"command":"/antigo/varzea"}}}"#).unwrap();
    fs::write(proj.join(".github/hooks/varzea.json"), "{}").unwrap();
    fs::write(proj.join(".github/skills/varzea/SKILL.md"), "antiga").unwrap();
    assert!(connect(&d, &brain, &proj, "copilot", &[]).status.success());
    assert!(read(proj.join(".vscode/mcp.json"))["servers"].get("varzea").is_none());
    assert!(read(proj.join(".github/mcp.json"))["mcpServers"].get("varzea").is_none());
    assert!(!proj.join(".github/hooks/varzea.json").exists());
    assert!(!proj.join(".github/skills/varzea").exists());
}

fn text(p: std::path::PathBuf) -> String {
    fs::read_to_string(p).unwrap()
}

fn tree(p: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![p.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap().flatten() {
            if e.path().is_dir() {
                stack.push(e.path());
            }
            out.push(e.path().strip_prefix(p).unwrap().to_string_lossy().into_owned());
        }
    }
    out.sort();
    out
}

fn ok(home: &std::path::Path, args: &[&str]) -> String {
    let o = run(home, args);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn disconnect_leaves_a_project_as_it_was() {
    let d = TempDir::new("off-clean");
    let brain = init_brain(&d);
    let b = brain.to_str().unwrap();
    for client in ["claude-code", "copilot"] {
        let proj = d.join(client);
        fs::create_dir_all(&proj).unwrap();
        let p = proj.to_str().unwrap();
        ok(d.path(), &["connect", client, "--brain", b, "--project", p]);
        assert!(!tree(&proj).is_empty());
        let printed = ok(d.path(), &["disconnect", client, "--project", p, "--print"]);
        assert!(printed.contains("mudariam"), "{printed}");
        assert!(!tree(&proj).is_empty(), "--print must not change anything");
        ok(d.path(), &["disconnect", client, "--project", p]);
        assert!(tree(&proj).is_empty(), "{:?}", tree(&proj));
        let again = ok(d.path(), &["disconnect", client, "--project", p]);
        assert!(again.contains("não estava conectado"), "{again}");
    }
}

#[test]
fn disconnect_keeps_everything_that_is_not_ours() {
    let d = TempDir::new("off-mine");
    let brain = init_brain(&d);
    let proj = d.join("proj");
    fs::create_dir_all(proj.join(".claude/skills/minha")).unwrap();
    fs::write(proj.join(".claude/skills/minha/SKILL.md"), "minha").unwrap();
    fs::write(proj.join(".mcp.json"), "{\"mcpServers\":{\"outro\":{\"command\":\"x\"}}}").unwrap();
    fs::write(
        proj.join(".claude/settings.json"),
        "{\"permissions\":{\"allow\":[\"Read\"]},\"hooks\":{\"Stop\":[{\"hooks\":[{\"type\":\"command\",\"command\":\"meu-script\"}]}]}}",
    )
    .unwrap();
    let (b, p) = (brain.to_str().unwrap(), proj.to_str().unwrap());
    let before = text(brain.join("brain.toml"));
    for _ in 0..3 {
        ok(d.path(), &["connect", "claude-code", "--brain", b, "--project", p]);
        ok(d.path(), &["disconnect", "claude-code", "--project", p]);
    }
    let mcp = text(proj.join(".mcp.json"));
    assert!(mcp.contains("outro") && !mcp.contains("omnimind"), "{mcp}");
    let settings = text(proj.join(".claude/settings.json"));
    assert!(settings.contains("meu-script") && settings.contains("permissions"), "{settings}");
    assert!(!settings.contains("omnimind"), "{settings}");
    assert!(proj.join(".claude/skills/minha/SKILL.md").exists());
    assert!(!proj.join(".claude/skills/omnimind").exists());
    assert_eq!(text(brain.join("brain.toml")), before);
    let copies: Vec<_> = tree(&proj).into_iter().filter(|f| f.contains("backup")).collect();
    assert_eq!(copies.len(), 2, "one per file, however many cycles: {copies:?}");
}

#[test]
fn disconnect_refuses_what_it_cannot_read_and_changes_nothing() {
    let d = TempDir::new("off-refuse");
    let brain = init_brain(&d);
    let proj = d.join("proj");
    fs::create_dir_all(proj.join(".vscode")).unwrap();
    let (b, p) = (brain.to_str().unwrap(), proj.to_str().unwrap());
    ok(d.path(), &["connect", "copilot", "--brain", b, "--project", p]);
    let commented = format!("// meu comentário\n{}", text(proj.join(".vscode/mcp.json")));
    fs::write(proj.join(".vscode/mcp.json"), &commented).unwrap();
    let before = tree(&proj);
    let o = run(d.path(), &["disconnect", "copilot", "--project", p]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("comentários"));
    assert_eq!(tree(&proj), before);
    assert_eq!(text(proj.join(".vscode/mcp.json")), commented);
}

// Symlink de pasta, do jeito do Unix: no Windows exige privilégio e outra chamada.
#[cfg(unix)]
#[test]
fn disconnect_does_not_follow_a_symlink_out_of_the_project() {
    let d = TempDir::new("off-link");
    let brain = init_brain(&d);
    let proj = d.join("proj");
    let elsewhere = d.join("elsewhere");
    fs::create_dir_all(&proj).unwrap();
    let (b, p) = (brain.to_str().unwrap(), proj.to_str().unwrap());
    ok(d.path(), &["connect", "claude-code", "--brain", b, "--project", p]);
    fs::rename(proj.join(".claude"), &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, proj.join(".claude")).unwrap();
    let o = run(d.path(), &["disconnect", "claude-code", "--project", p]);
    assert!(!o.status.success());
    assert!(elsewhere.join("settings.json").exists());
    assert!(elsewhere.join("skills/omnimind/SKILL.md").exists());
}

#[test]
fn global_disconnect_stays_inside_the_test_home() {
    let d = TempDir::new("off-global");
    let brain = init_brain(&d);
    let b = brain.to_str().unwrap();
    fs::write(d.join(".claude.json"), "{\"numStartups\":3}").unwrap();
    ok(d.path(), &["connect", "claude-code", "--brain", b, "--global"]);
    ok(d.path(), &["disconnect", "claude-code", "--global"]);
    let kept = text(d.join(".claude.json"));
    assert!(kept.contains("numStartups") && !kept.contains("omnimind"), "{kept}");
    assert!(d.join(".claude").exists(), "the home folder itself is never removed");
}

#[test]
fn connecting_keeps_the_order_of_a_config_it_does_not_own() {
    let d = TempDir::new("order");
    let brain = init_brain(&d);
    let proj = d.join("proj");
    fs::create_dir_all(&proj).unwrap();
    let mine = "{\n  \"zeta\": 1,\n  \"alpha\": 2,\n  \"mcpServers\": {\n    \"outro\": {\n      \"command\": \"x\"\n    }\n  }\n}\n";
    fs::write(proj.join(".mcp.json"), mine).unwrap();
    let (b, p) = (brain.to_str().unwrap(), proj.to_str().unwrap());
    ok(d.path(), &["connect", "claude-code", "--brain", b, "--project", p]);
    let after = text(proj.join(".mcp.json"));
    assert!(after.find("zeta").unwrap() < after.find("alpha").unwrap(), "{after}");
    ok(d.path(), &["disconnect", "claude-code", "--project", p]);
    assert_eq!(text(proj.join(".mcp.json")), mine, "disconnect restores the file exactly");
}
