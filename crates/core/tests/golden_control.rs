//! Controle positivo: com o índice desligado, o golden set tem de falhar.
//! Fica em binário próprio porque a variável de ambiente vale para o processo inteiro.

mod common;

use common::TempDir;
use std::path::Path;
use omnimind_core::{Brain, SearchQuery};

#[test]
fn golden_fails_when_index_is_off() {
    std::env::set_var("OMNIMIND_TEST_NO_INDEX", "1");
    let d = TempDir::new("control");
    let b = Brain::init(d.join("work"), "work").unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/brains/work");
    b.import_markdown(&src).unwrap();
    assert!(b.memories().unwrap().len() >= 9, "o disco continua cheio");
    for q in ["jsprit", "rabbitmq", "canary", "INC-042"] {
        let r = b.search(&SearchQuery::new(q)).unwrap();
        assert!(r.hits.is_empty(), "sem índice nada pode voltar: {q}");
    }
}
