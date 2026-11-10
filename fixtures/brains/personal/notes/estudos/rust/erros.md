---
title: Tratamento de erros em Rust
aliases: [error handling, anyhow, thiserror]
tags: [rust, estudo, decisao]
---
# Tratamento de erros em Rust

Depois de brigar com `Box<dyn Error>`, regra que adotei em todos os meus projetos:

- Biblioteca usa `thiserror`, com um enum próprio, para quem chama poder reagir a cada caso.
- Binário usa `anyhow`, com `.context("...")` em cada passo, porque ali o erro só vai para a tela.
- `unwrap()` só em teste e em exemplo.

No Cuscuz isso quer dizer: `cuscuz-core` com `thiserror` e `cuscuz-cli` com `anyhow`. Ver [[Stack do Cuscuz]].
