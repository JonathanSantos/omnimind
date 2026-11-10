---
title: Checklist de release em Rust
aliases: [release checklist, publicar versão]
tags: [rust, procedimento]
---
# Checklist de release em Rust

Sempre nessa ordem, antes de marcar uma versão:

1. `cargo fmt --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --workspace`
4. Atualizar `CHANGELOG.md` e a versão em `Cargo.toml`.
5. `cargo build --release` e abrir o binário de verdade, uma vez.
6. Criar a tag `vX.Y.Z` só depois do passo 5.

Na versão 0.2 do Cuscuz pulei o passo 5 e saiu um binário sem o ícone.
