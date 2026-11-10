//! Gera um brain fictício grande, para medir as travas. Uso: make-brain <pasta> <notas> <memórias>
use std::time::Instant;
use omnimind_core::{Actor, Brain, Change, MemoryDraft, MemoryType};

const WORDS: [&str; 40] = ["decisão", "rota", "preço", "marca", "teste", "ação", "hábito", "revisão", "fluxo", "sessão", "índice", "órgão", "coração", "cache", "deploy", "contrato", "entrega", "cliente", "projeto", "memória", "sistema", "equipe", "reunião", "custo", "prazo", "risco", "piloto", "bairro", "receita", "orçamento", "treino", "leitura", "viagem", "sono", "café", "manual", "plano", "objetivo", "estudo", "ideia"];
const FOLDERS: [&str; 12] = ["rota-certa", "arquitetura", "clientes", "leituras", "saude", "financas", "marca", "viagens", "equipe", "estudos", "receitas", "diario"];

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = &a[1];
    let notes: usize = a[2].parse().unwrap();
    let mems: usize = a[3].parse().unwrap();
    let b = Brain::init(dir, "Grande").expect("init");
    let mut s = 0x9E3779B97F4A7C15u64;
    let mut rnd = move |n: usize| {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s % n as u64) as usize
    };
    let t = Instant::now();
    let mut titles = Vec::with_capacity(notes);
    for i in 0..notes {
        let f = FOLDERS[rnd(FOLDERS.len())];
        let title = format!("{} {} {i}", WORDS[rnd(40)], WORDS[rnd(40)]);
        let path = format!("{dir}/notes/{f}/{}.md", title.replace(' ', "-"));
        std::fs::create_dir_all(format!("{dir}/notes/{f}")).unwrap();
        let mut text = format!("# {title}\n\n");
        for _ in 0..(40 + rnd(120)) {
            text.push_str(WORDS[rnd(40)]);
            text.push(' ');
            if rnd(14) == 0 {
                text.push_str(".\n\n");
            }
        }
        if i > 0 {
            for _ in 0..rnd(4) {
                let t: &String = &titles[rnd(i)];
                text.push_str(&format!("\n\nVeja [[{t}]]."));
            }
        }
        std::fs::write(path, text).unwrap();
        titles.push(title);
    }
    println!("notes in {:?}", t.elapsed());
    let t = Instant::now();
    let types = MemoryType::ALL;
    for i in 0..mems {
        let d = MemoryDraft {
            mem_type: Some(types[rnd(5)]),
            title: format!("{} {} {} {i}", WORDS[rnd(40)], WORDS[rnd(40)], WORDS[rnd(40)]),
            summary: format!("Resumo sobre {} e {} usado no projeto.", WORDS[rnd(40)], WORDS[rnd(40)]),
            body: (0..24).map(|_| WORDS[rnd(40)]).collect::<Vec<_>>().join(" "),
            tags: vec![format!("project/{}", FOLDERS[rnd(FOLDERS.len())])],
            ..Default::default()
        };
        // Uma sessão a cada 30: o teto de crescimento por sessão continua valendo.
        let actor = Actor::agent("claude-code", &format!("gerador-{}", i / 30));
        let _ = b.apply(Change::Write(d), actor);
    }
    println!("memories in {:?}", t.elapsed());
    b.flush().unwrap();
}
