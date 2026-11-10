//! `omnimind connect` e `disconnect`: a parte de linha de comando. A lógica fica no núcleo, junto do app.

use crate::Args;
use omnimind_core::connect::{self, Options};
use std::path::PathBuf;

const USAGE: &str = "omnimind connect|disconnect claude-code|copilot [--project PASTA] [--print] [--global]";

fn options(args: &Args) -> Result<Options, i32> {
    let Some(target) = args.positional.first() else {
        eprintln!("Uso: {USAGE} [--brain PASTA]");
        return Err(2);
    };
    let project = args.flag("project").map_or_else(
        || std::env::current_dir().unwrap_or_default(),
        PathBuf::from,
    );
    let Ok(project) = project.canonicalize() else {
        eprintln!("Erro: pasta do projeto não encontrada.");
        return Err(1);
    };
    Ok(Options {
        target: target.clone(),
        project,
        home: omnimind_core::util::home_dir().unwrap_or_default(),
        global: args.has("global"),
        print: args.has("print"),
    })
}

pub fn run(args: &Args) -> i32 {
    let o = match options(args) {
        Ok(o) => o,
        Err(code) => return code,
    };
    let Some(brain) = args.brain_path() else {
        eprintln!("Erro: nenhum cofre. Use --brain ou OMNIMIND_BRAIN.");
        return 1;
    };
    if !brain.join("brain.toml").is_file() {
        eprintln!("Erro: cofre não encontrado.");
        return 1;
    }
    let brain = brain.canonicalize().unwrap_or(brain);
    let bin = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "omnimind".into());
    match connect::connect(&o, &bin, &brain.to_string_lossy(), &mut |l| println!("{l}")) {
        Ok(changed) => {
            println!("{changed} arquivos {}", if o.print { "mudariam" } else { "alterados" });
            0
        }
        Err(e) => {
            eprintln!("Erro: {e}");
            1
        }
    }
}

pub fn run_disconnect(args: &Args) -> i32 {
    let o = match options(args) {
        Ok(o) => o,
        Err(code) => return code,
    };
    match connect::disconnect(&o, &mut |l| println!("{l}")) {
        Ok(0) => {
            println!("Este projeto não estava conectado.");
            0
        }
        Ok(changed) if o.print => {
            println!("{changed} arquivos mudariam");
            0
        }
        Ok(changed) => {
            println!("{changed} arquivos alterados. O cofre não mudou. Vale a partir da próxima sessão.");
            0
        }
        Err(e) => {
            eprintln!("Erro: {e}");
            1
        }
    }
}
