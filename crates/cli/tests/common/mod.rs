#![allow(dead_code)]

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

pub const BIN: &str = env!("CARGO_BIN_EXE_omnimind");
static N: AtomicU64 = AtomicU64::new(0);

pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "vzc-{tag}-{}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros()
        ));
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p.canonicalize().unwrap())
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn join(&self, s: &str) -> PathBuf {
        self.0.join(s)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Comando com HOME temporário: nenhum teste toca a configuração real.
pub fn cmd(home: &Path) -> Command {
    let mut c = Command::new(BIN);
    c.env("HOME", home)
        .env_remove("OMNIMIND_BRAIN")
        .env_remove("CLAUDE_PROJECT_DIR");
    c
}

pub fn run(home: &Path, args: &[&str]) -> Output {
    cmd(home).args(args).stdin(Stdio::null()).output().unwrap()
}

pub fn init_brain(d: &TempDir) -> PathBuf {
    let p = d.join("brain");
    let o = run(d.path(), &["init", p.to_str().unwrap(), "--name", "Teste"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    p
}

/// Servidor MCP real, com PID próprio. `Drop` só mata o filho que criou.
pub struct Mcp {
    pub child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    next: u64,
}

impl Mcp {
    pub fn spawn(home: &Path, brain: &Path, extra: &[&str]) -> Mcp {
        let mut child = cmd(home)
            .args(["mcp", "--brain", brain.to_str().unwrap()])
            .args(extra)
            .current_dir(home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Mcp {
            child,
            stdin,
            stdout,
            next: 1,
        }
    }

    pub fn send_raw(&mut self, bytes: &[u8]) {
        let s = self.stdin.as_mut().unwrap();
        s.write_all(bytes).unwrap();
        s.flush().unwrap();
    }

    pub fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert!(!line.is_empty(), "server closed");
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {line}"))
    }

    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next;
        self.next += 1;
        let msg = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        self.send_raw(format!("{msg}\n").as_bytes());
        let r = self.recv();
        assert_eq!(r["id"], json!(id), "{r}");
        r
    }

    pub fn call(&mut self, tool: &str, args: Value) -> Value {
        let r = self.request("tools/call", json!({"name": tool, "arguments": args}));
        assert!(r.get("result").is_some(), "{r}");
        r["result"]["structuredContent"].clone()
    }

    pub fn close(mut self) -> std::process::ExitStatus {
        drop(self.stdin.take());
        self.child.wait().unwrap()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        if self.stdin.is_some() {
            drop(self.stdin.take());
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
