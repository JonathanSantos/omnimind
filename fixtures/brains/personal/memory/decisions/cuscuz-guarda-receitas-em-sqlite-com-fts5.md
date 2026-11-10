---
id: 01M69V21DXRR22VJ1XAXXG60Q0
type: decision
title: Cuscuz guarda receitas em SQLite com FTS5
summary: Em 29/10/2026 o Cuscuz trocou um JSON por receita por um único arquivo SQLite com FTS5, porque a busca por ingrediente levava ~2 s com 340 receitas; backup é cópia semanal do .db.
aliases:
- armazenamento do Cuscuz
- Cuscuz storage
- banco de dados Cuscuz
- SQLite FTS5
tags:
- projeto/cuscuz
- decisao
edges: []
source:
  note: notes/projetos/cuscuz/stack.md
  hash: 8ab4c7b5
  session: 01M69V1GT33ZH8E95X01SFJA6B
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:40:14Z
updated_at: 2026-10-31T20:43:00Z
---

Decisão de 2026-10-29. Substitui a escolha de 2026-10-26 (um arquivo JSON por receita), que está revogada.

Motivo: com 340 receitas importadas, a busca por ingrediente em JSON levava quase 2 s no notebook e o app relia a pasta inteira a cada abertura.

Escolha: SQLite com FTS5 em um arquivo só.
Custo aceito: perde o diff bonito no git.
Backup: copiar o arquivo .db uma vez por semana.
O modelo de dados não muda.

Stack geral: backend em Rust, interface em Tauri.
