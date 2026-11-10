---
id: 01M69V4Q3ZNF5QRVG7813E3B8P
type: episode
title: 'INC-042: rota por via interditada (29/10/2026)'
summary: 'SEV2, 7h10–9h40: 14 caminhões enviados pela Fernão Dias interditada, 37 entregas atrasadas e 6 recusas de janela dura; causa: cache de matriz de 6h, sem consulta a boletins de trânsito, e mapa OSRM de 3 dias.'
aliases:
- INC-042
- incidente da matriz
- road closure incident
- Fernão Dias
- retro INC-042
tags:
- incidente
- projeto/rota-certa
edges: []
source:
  note: notes/incidentes/2026-10-29-inc-042.md
  hash: ae603f6b
  session: 01M69V345Z9XT1ETEFPJADPQ25
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:41:42Z
updated_at: 2026-10-31T20:43:00Z
---

Incidente de 2026-10-29, severidade 2, 7h10–9h40.

Impacto: 14 caminhões por trecho da Fernão Dias interditado desde as 22h por obra; 37 entregas atrasaram; 6 clientes de janela dura recusaram a carga.

Causa raiz: matriz vinda do cache de 6h e nenhum passo do fluxo consultava boletins de trânsito; OSRM com mapa de 3 dias antes.

Correção imediata: Ricardo refez as rotas à mão; o procedimento de reprocessar roteiros foi usado pela primeira vez, ainda improvisado.

Retro (30/10, blameless): bom — motoristas ligaram ao despacho em minutos e Ricardo replanejou. Ruim — pipeline sem conhecimento de interdições e cache escondeu a mudança. Ações: Davi reduz TTL e adiciona invalidação; Lívia assina o feed de boletins no `otimizador` até 07/11; runbook de reprocessamento escrito.
