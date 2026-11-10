//! Projeto é o que uma nota declara: grupo, pastas e apelidos. A pasta da sessão só leva até ele.
mod common;

use common::*;
use omnimind_core::{Actor, Brain, Change, MemoryDraft, MemoryType, Outcome, Place};

fn note(b: &Brain, rel: &str, text: &str) {
    let p = b.root().join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
    b.reconcile().unwrap();
    b.flush().unwrap();
}

fn at(b: &Brain, cwd: &str, kind: MemoryType, title: &str, summary: &str) -> String {
    let actor = Actor::agent("claude-code", "s1").with_cwd(cwd);
    let d = MemoryDraft { mem_type: Some(kind), title: title.into(), summary: summary.into(), ..Default::default() };
    let a = b.apply(Change::Write(d), actor).unwrap();
    a.id.unwrap_or_else(|| panic!("{:?}", a.outcome))
}

fn tags(b: &Brain, id: &str) -> Vec<String> {
    b.memories().unwrap().into_iter().find(|m| m.id == id).expect("memory").tags
}

const VITROLA: &str = "# Vitrola\n\nColeção de discos.\n\n- **Grupo:** Projetos\n- **Pasta:** `~/Developer/projects/toca-discos`\n";

#[test]
fn the_project_note_names_the_project_whatever_the_folder_is_called() {
    let (_d, b) = brain("proj-note");
    // Antes da nota existir, a pasta vale como projeto provisório.
    let old = at(&b, "/Users/x/Developer/projects/toca-discos", MemoryType::Decision, "Grade em colunas", "A estante usa grade em colunas.");
    assert_eq!(tags(&b, &old), vec!["project/toca-discos"]);
    note(&b, "notes/projetos/vitrola.md", VITROLA);
    let p = b.projects().unwrap();
    assert_eq!(p.list.len(), 1);
    assert_eq!(p.place("/Users/x/Developer/projects/toca-discos"), Place::Project("vitrola".into()));
    let new = at(&b, "/Users/x/Developer/projects/toca-discos/web", MemoryType::Decision, "Imagens em WebP", "A capa é convertida para WebP.");
    assert_eq!(tags(&b, &new), vec!["project/vitrola"]);

    // O resumo trata o nome antigo e o novo como o mesmo projeto, nos dois caminhos.
    for text in [
        b.briefing(Some("/Users/x/Developer/projects/toca-discos"), "x").unwrap().text,
        omnimind_core::briefing_light(b.root(), Some("/Users/x/Developer/projects/toca-discos"), "x").unwrap().text,
    ] {
        assert!(text.contains("Projeto atual: Vitrola"), "{text}");
        assert!(text.contains("A estante usa grade em colunas"), "{text}");
        assert!(text.contains("A capa é convertida para WebP"), "{text}");
        assert!(!text.contains("Outros projetos"), "{text}");
    }
    // Em outro projeto, as duas são de fora.
    let text = b.briefing(Some("/Users/x/Developer/projects/trilho"), "x").unwrap().text;
    assert!(text.contains("Outros projetos: não valem aqui"), "{text}");
    assert!(!text.contains("A estante usa grade em colunas"), "{text}");
}

#[test]
fn a_throwaway_folder_never_becomes_the_project_of_a_rule() {
    let (_d, b) = brain("proj-scratch");
    note(&b, "notes/projetos/trilho.md", "# trilho\n\n- **Grupo:** Projetos\n- **Pasta:** `~/Developer/projects/trilho`\n");
    let scratch = "/Users/x/Developer/projects/scratch-2026-01-01-abc123";
    // Regra dita numa pasta descartável é geral: vale em todo projeto.
    let rule = at(&b, scratch, MemoryType::Procedure, "Registrar só a decisão que muda o contrato", "Registra só a decisão que muda o contrato.");
    assert!(tags(&b, &rule).is_empty(), "{:?}", tags(&b, &rule));
    // Título que nomeia um projeto com nota vai para ele.
    let named = at(&b, scratch, MemoryType::Decision, "trilho: índice das decisões 1 a 14", "Índice das decisões do trilho.");
    assert_eq!(tags(&b, &named), vec!["project/trilho"]);
    // Decisão solta guarda o nome da pasta: não vale em todo lugar até a pessoa decidir.
    let loose = at(&b, scratch, MemoryType::Decision, "Gerador em processo separado", "O gerador roda num processo separado.");
    assert_eq!(tags(&b, &loose), vec!["project/scratch-2026-01-01-abc123"]);

    let text = b.briefing(Some("/Users/x/Developer/projects/trilho"), "x").unwrap().text;
    assert!(text.contains("Registra só a decisão que muda o contrato"), "regra geral entra por extenso: {text}");
    assert!(text.contains("[trilho] Índice das decisões do trilho"), "{text}");
    assert!(!text.contains("O gerador de vídeos é um binário em Go"), "{text}");
    // Numa pasta descartável não existe "projeto atual", e nada é de fora.
    let text = b.briefing(Some(scratch), "x").unwrap().text;
    assert!(!text.contains("Projeto atual"), "{text}");
    assert!(!text.contains("Outros projetos"), "{text}");
}

#[test]
fn a_memory_distilled_from_a_project_note_belongs_to_that_project() {
    let (_d, b) = brain("proj-origin");
    note(&b, "notes/projetos/cuscuz.md", "# cuscuz\n\nTexto do projeto.\n\n- **Grupo:** Trabalho\n- **Pasta:** `~/Developer/projects/cuscuz`\n");
    // Destilar exige ter aberto a nota inteira.
    b.read_item(&omnimind_core::ReadRequest { target: "notes/projetos/cuscuz.md".into(), ..Default::default() }).unwrap();
    let actor = Actor::agent("claude-code", b.session()).with_cwd("/Users/x/Developer/projects/outro-lugar");
    let d = MemoryDraft { mem_type: Some(MemoryType::Decision), title: "Receitas em SQLite".into(), summary: "O cuscuz guarda receitas em SQLite.".into(), note: Some("notes/projetos/cuscuz.md".into()), ..Default::default() };
    let a = b.apply(Change::Write(d), actor).unwrap();
    let id = a.id.clone().unwrap_or_else(|| panic!("{:?}", a.outcome));
    assert_eq!(tags(&b, &id), vec!["project/cuscuz"]);
}

#[test]
fn a_memory_has_at_most_one_project() {
    let (_d, b) = brain("proj-one");
    note(&b, "notes/projetos/trilho.md", "# trilho\n\n- **Grupo:** Projetos\n- **Pasta:** `~/Developer/projects/trilho`\n");
    let actor = Actor::agent("claude-code", "s1").with_cwd("/Users/x/Developer/projects/peneira");
    // A IA escreveu a tag de um projeto com nota: vale ela, e a pasta não soma outra.
    let d = MemoryDraft { mem_type: Some(MemoryType::Decision), title: "Mapa sem conexão".into(), summary: "O mapa abre sem conexão.".into(), tags: vec!["project/trilho".into(), "rust".into()], ..Default::default() };
    let a = b.apply(Change::Write(d), actor.clone()).unwrap();
    assert_eq!(tags(&b, a.id.as_deref().unwrap()), vec!["project/trilho", "rust"]);
    // Tag de projeto que ninguém conhece perde para a pasta da sessão.
    let d = MemoryDraft { mem_type: Some(MemoryType::Decision), title: "Peneira por lote".into(), summary: "A peneira processa por lote.".into(), tags: vec!["project/inventado".into()], ..Default::default() };
    let a = b.apply(Change::Write(d), actor).unwrap();
    assert_eq!(tags(&b, a.id.as_deref().unwrap()), vec!["project/peneira"]);
}

#[test]
fn only_the_person_moves_a_memory_and_general_means_everywhere() {
    let (_d, b) = brain("proj-move");
    note(&b, "notes/projetos/vitrola.md", VITROLA);
    let id = at(&b, "/Users/x/Developer/projects/scratch-1", MemoryType::Decision, "Confirmar antes de publicar", "Confirma com a pessoa antes de publicar.");
    assert_eq!(tags(&b, &id), vec!["project/scratch-1"]);
    let hash = |b: &Brain| b.memories().unwrap().into_iter().find(|m| m.id == id).unwrap().hash;

    let refused = b.apply(Change::SetProject { id: id.clone(), base_hash: Some(hash(&b)), project: None }, agent(&b)).unwrap();
    assert!(matches!(refused.outcome, Outcome::Rejected { ref reason, .. } if reason == "project_requires_person"), "{:?}", refused.outcome);
    let stale = b.apply(Change::SetProject { id: id.clone(), base_hash: Some("velho".into()), project: None }, Actor::person()).unwrap();
    assert!(matches!(stale.outcome, Outcome::Rejected { .. }));

    // Pelo apelido da pasta chega ao projeto da nota.
    let moved = b.apply(Change::SetProject { id: id.clone(), base_hash: Some(hash(&b)), project: Some("toca-discos".into()) }, Actor::person()).unwrap();
    assert_eq!(moved.outcome, Outcome::Written);
    assert_eq!(tags(&b, &id), vec!["project/vitrola"]);
    let same = b.apply(Change::SetProject { id: id.clone(), base_hash: Some(hash(&b)), project: Some("vitrola".into()) }, Actor::person()).unwrap();
    assert_eq!(same.outcome, Outcome::Unchanged);

    let general = b.apply(Change::SetProject { id: id.clone(), base_hash: Some(hash(&b)), project: None }, Actor::person()).unwrap();
    assert_eq!(general.outcome, Outcome::Written);
    assert!(tags(&b, &id).is_empty());
    let text = b.briefing(Some("/Users/x/Developer/projects/trilho"), "x").unwrap().text;
    assert!(text.contains("Confirma com a pessoa antes de publicar"), "{text}");
    // A mudança fica no diário e pode ser desfeita como qualquer outra.
    let undone = b.undo(general.event.as_deref().unwrap());
    assert!(undone.is_ok(), "{:?}", undone.err());
    assert_eq!(tags(&b, &id), vec!["project/vitrola"]);
}

#[test]
fn notes_on_the_same_subject_are_linked_without_any_tag_or_wikilink() {
    let (_d, b) = brain("similar");
    note(&b, "notes/a/torra.md", "# Torra clara\n\nCafé de torra clara pede moagem mais fina no coador e água a 94 graus.\n");
    note(&b, "notes/b/coador.md", "# Coador de pano\n\nNo coador, a moagem fina e a água a 94 graus realçam a torra clara do café.\n");
    note(&b, "notes/c/frete.md", "# Tabela de frete\n\nO frete por faixa de CEP usa a tabela da transportadora e arredonda para cima.\n");
    note(&b, "notes/d/deploy.md", "# Deploy\n\nMigração do banco primeiro, aplicação depois, com janela às terças.\n");
    note(&b, "notes/e/rota.md", "# Roteirização\n\nO motor calcula as rotas do piloto com janelas de entrega por bairro.\n");
    let g = b.graph().unwrap();
    let similar: Vec<(&str, &str)> = g.edges.iter().filter(|e| e.kind == "similar").map(|e| (e.from.as_str(), e.to.as_str())).collect();
    assert!(similar.contains(&("note:notes/a/torra.md", "note:notes/b/coador.md")), "{similar:?}");
    assert!(!similar.iter().any(|(a, b)| a.contains("frete") && b.contains("torra") || a.contains("torra") && b.contains("frete")), "{similar:?}");
    // Ligação já declarada não ganha uma segunda, por semelhança.
    note(&b, "notes/a/torra.md", "# Torra clara\n\nCafé de torra clara pede moagem mais fina no coador e água a 94 graus. Veja [[Coador de pano]].\n");
    let g = b.graph().unwrap();
    let between: Vec<&str> = g.edges.iter().filter(|e| e.from.contains("torra") && e.to.contains("coador") || e.from.contains("coador") && e.to.contains("torra")).map(|e| e.kind.as_str()).collect();
    assert_eq!(between, vec!["wikilink"], "{between:?}");
}

#[test]
fn the_plan_proposes_what_the_rules_would_have_done_and_changes_nothing() {
    let (_d, b) = brain("proj-plan");
    let scratch = "/Users/x/Developer/projects/scratch-9";
    // Antes das notas existirem, tudo isto ganhou o nome da pasta descartável.
    let loose = at(&b, scratch, MemoryType::Decision, "Gerador em processo separado", "O gerador roda num processo separado.");
    let named = at(&b, scratch, MemoryType::Decision, "trilho: índice das decisões", "Índice das decisões do trilho.");
    let old = at(&b, "/Users/x/Developer/projects/toca-discos", MemoryType::Decision, "Grade em colunas", "A estante usa grade em colunas.");
    assert_eq!(tags(&b, &named), vec!["project/scratch-9"]);
    note(&b, "notes/projetos/trilho.md", "# trilho\n\n- **Grupo:** Projetos\n- **Pasta:** `~/Developer/projects/trilho`\n");
    note(&b, "notes/projetos/vitrola.md", VITROLA);

    let plan = b.project_plan().unwrap();
    let of = |id: &str| plan.iter().find(|m| m.id == id).map(|m| (m.from.clone(), m.to.clone()));
    assert_eq!(of(&named), Some((Some("scratch-9".into()), Some("trilho".into()))), "{plan:?}");
    assert_eq!(of(&old), Some((Some("toca-discos".into()), Some("vitrola".into()))), "apelido vira o nome da nota");
    assert_eq!(of(&loose), None, "decisão solta espera a pessoa: {plan:?}");
    // Propor não altera nada.
    assert_eq!(tags(&b, &named), vec!["project/scratch-9"]);

    for m in &plan {
        let done = b.apply(Change::SetProject { id: m.id.clone(), base_hash: Some(m.hash.clone()), project: m.to.clone() }, Actor::person()).unwrap();
        assert_eq!(done.outcome, Outcome::Written, "{m:?}");
    }
    assert_eq!(tags(&b, &named), vec!["project/trilho"]);
    assert_eq!(tags(&b, &old), vec!["project/vitrola"]);
    assert!(b.project_plan().unwrap().is_empty(), "aplicado, não sobra o que propor");
}

#[test]
fn the_hook_sees_a_new_project_note_without_waiting_for_a_snapshot() {
    let (_d, b) = brain("proj-cache");
    // Nota criada pelo app: ninguém gravou instantâneo ainda.
    b.save_note("notes/projetos/vitrola.md", VITROLA, None).unwrap();
    assert_eq!(b.projects().unwrap().list.len(), 1);
    let text = omnimind_core::briefing_light(b.root(), Some("/Users/x/Developer/projects/toca-discos"), "x").unwrap().text;
    assert!(text.contains("Projeto atual: Vitrola"), "{text}");
    // Sem mudança na lista, o arquivo não é regravado.
    let file = b.root().join(".omnimind/projects.json");
    let before = std::fs::metadata(&file).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    b.projects().unwrap();
    assert_eq!(std::fs::metadata(&file).unwrap().modified().unwrap(), before);
}

#[test]
fn a_whole_project_moves_at_once_and_archived_memories_stay() {
    let (_d, b) = brain("proj-bulk");
    note(&b, "notes/projetos/vitrola.md", VITROLA);
    let scratch = "/Users/x/Developer/projects/scratch-7";
    let a = at(&b, scratch, MemoryType::Decision, "Gerador em processo separado", "O gerador roda num processo separado.");
    let c = at(&b, scratch, MemoryType::Concept, "Nome do produto difere da pasta", "O nome do produto nem sempre é o da pasta.");
    let gone = at(&b, scratch, MemoryType::Episode, "Protótipo descartado", "O primeiro protótipo foi descartado.");
    let h = b.memories().unwrap().into_iter().find(|m| m.id == gone).unwrap().hash;
    b.apply(Change::Archive { id: gone.clone(), base_hash: Some(h), reason: None }, Actor::person()).unwrap();

    assert_eq!(b.move_project("scratch-7", None, &agent(&b)).unwrap(), 0, "a IA não move memória de projeto");
    assert_eq!(tags(&b, &a), vec!["project/scratch-7"]);
    assert_eq!(b.move_project("scratch-7", None, &Actor::person()).unwrap(), 2);
    assert!(tags(&b, &a).is_empty() && tags(&b, &c).is_empty());
    assert_eq!(tags(&b, &gone), vec!["project/scratch-7"], "arquivada fica como estava");
    assert_eq!(b.move_project("scratch-7", None, &Actor::person()).unwrap(), 0, "não sobrou o que mover");
    // E para um projeto com nota, pelo apelido.
    let d = at(&b, "/Users/x/Developer/projects/outro", MemoryType::Decision, "Capas em WebP", "As capas vão em WebP.");
    assert_eq!(b.move_project("outro", Some("toca-discos"), &Actor::person()).unwrap(), 1);
    assert_eq!(tags(&b, &d), vec!["project/vitrola"]);
}
