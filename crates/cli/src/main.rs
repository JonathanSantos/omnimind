//! Binário `omnimind`: CLI, servidor MCP stdio e hooks.

mod commands;
mod connect;
mod hooks;
mod mcp;

use std::collections::HashMap;
use std::path::PathBuf;

/// Argumentos: posicionais e `--chave valor`. Bandeiras sem valor valem `true`.
pub struct Args {
    pub positional: Vec<String>,
    pub flags: HashMap<String, String>,
}

const BOOL_FLAGS: &[&str] = &["json", "print", "global", "old", "dry-run"];

impl Args {
    pub fn parse(raw: &[String]) -> Args {
        let mut a = Args {
            positional: Vec::new(),
            flags: HashMap::new(),
        };
        let mut i = 0;
        while i < raw.len() {
            let s = &raw[i];
            if let Some(name) = s.strip_prefix("--") {
                if let Some((k, v)) = name.split_once('=') {
                    a.flags.insert(k.to_string(), v.to_string());
                } else if BOOL_FLAGS.contains(&name) || i + 1 >= raw.len() {
                    a.flags.insert(name.to_string(), "true".to_string());
                } else {
                    a.flags.insert(name.to_string(), raw[i + 1].clone());
                    i += 1;
                }
            } else {
                a.positional.push(s.clone());
            }
            i += 1;
        }
        a
    }

    pub fn flag(&self, k: &str) -> Option<&str> {
        self.flags.get(k).map(String::as_str)
    }

    pub fn has(&self, k: &str) -> bool {
        self.flags.contains_key(k)
    }

    /// `--brain`, `OMNIMIND_BRAIN`, ou o primeiro `brain.toml` subindo do diretório atual.
    pub fn brain_path(&self) -> Option<PathBuf> {
        if let Some(b) = self.flag("brain") {
            return Some(PathBuf::from(b));
        }
        if let Some(b) = std::env::var_os("OMNIMIND_BRAIN") {
            return Some(PathBuf::from(b));
        }
        let mut dir = std::env::current_dir().ok();
        while let Some(d) = dir {
            if d.join("brain.toml").is_file() {
                return Some(d);
            }
            dir = d.parent().map(PathBuf::from);
        }
        None
    }
}

const USAGE: &str = "omnimind <comando>

  init PASTA [--name NOME]        cria um cofre
  doctor | lint                   confere o cofre (lint sai com 1 se houver erro)
  search CONSULTA [--limit N]     busca em notas e memórias
  read ID|CAMINHO                 lê uma memória ou nota
  remember TEXTO [--type T]       grava uma memória em seu nome
  undo EVENTO | --session S       desfaz um evento ou uma sessão inteira
  log [--limit N]                 diário, o mais novo por último
  agenda | sleep | portrait       ciclo de vida
  export --out PASTA              exporta memórias, notas e diário
  import-markdown ORIGEM          importa uma pasta de Markdown
  mcp --brain PASTA [--client C]  servidor MCP por stdio
  hook EVENTO [--brain PASTA]     session-start|prompt|post-tool|pre-compact|stop|session-end
  connect claude-code|copilot [--project PASTA] [--brain PASTA] [--print] [--global]
  disconnect claude-code|copilot [--project PASTA] [--print] [--global]

Opções comuns: --brain PASTA, --json. OMNIMIND_BRAIN também escolhe o cofre.";

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = raw.first().cloned() else {
        println!("{USAGE}");
        return;
    };
    let args = Args::parse(&raw[1..]);
    let code = match cmd.as_str() {
        "mcp" => mcp::run(&args),
        "hook" => hooks::run(&args),
        "connect" => connect::run(&args),
        "disconnect" => connect::run_disconnect(&args),
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            0
        }
        "--version" | "version" => {
            println!("omnimind {}", env!("CARGO_PKG_VERSION"));
            0
        }
        other => commands::run(other, &args),
    };
    std::process::exit(code);
}
