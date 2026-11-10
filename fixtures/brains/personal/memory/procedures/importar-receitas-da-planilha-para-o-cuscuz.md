---
id: 01M69V264X15NY38JNSX6C8TE7
type: procedure
title: Importar receitas da planilha para o Cuscuz
summary: Exportar CSV em UTF-8, rodar `cuscuz import --dry-run`, corrigir unidades em unidades.toml, rodar de verdade e conferir 10 receitas aleatórias antes de apagar qualquer coisa.
aliases:
- migrar planilha
- import spreadsheet
- cuscuz import
tags:
- projeto/cuscuz
- procedimento
edges: []
source:
  note: notes/projetos/cuscuz/importar-receitas.md
  hash: f8bc737f
  session: 01M69V1GT33ZH8E95X01SFJA6B
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:40:19Z
updated_at: 2026-10-31T20:43:00Z
---

1. Exportar a planilha como CSV em UTF-8 (no formato padrão os acentos quebram).
2. `cuscuz import --dry-run receitas.csv` e olhar as unidades desconhecidas.
3. Corrigir em `unidades.toml`. "Copo americano" = 190 ml.
4. Rodar sem `--dry-run`.
5. Conferir 10 receitas ao acaso no app antes de apagar qualquer coisa.

Armadilha: receitas com rendimento por extenso ("serve seis") falharam na primeira rodada (12 receitas).
