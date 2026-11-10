---
title: Stack do Cuscuz
tags: [projeto/cuscuz, decisao]
---
# Stack do Cuscuz

## 2026-10-26

Backend em Rust, porque é o que estou estudando ([[Tratamento de erros em Rust]]). Interface em Tauri. Para guardar as receitas vou usar um arquivo JSON por receita: dá para versionar no git e abrir no editor. Sem banco de dados, é só um app pessoal.

## 2026-10-29

Mudei de ideia. Com 340 receitas importadas, a busca por ingrediente em arquivos JSON leva quase 2 segundos no notebook e o app inteiro relê a pasta a cada abertura. Vou trocar para SQLite com FTS5 em um arquivo só. Perco o diff bonito no git, mas ganho busca instantânea. Backup vira uma cópia do arquivo .db uma vez por semana.

A decisão de 26/10 sobre JSON está revogada. O modelo continua o de [[Modelo de dados das receitas]].
