---
id: 01M69V4KSNGBVC7X7KJ28MKX0Q
type: decision
title: 'Cache da matriz de distâncias: TTL 1h com invalidação por boletim'
summary: 'Matriz no Redis com TTL de 1 hora (era 6h), invalidada na hora quando chega boletim de interdição da central de trânsito; custo de ~2 min por rodada, aceito. Revisão pós INC-042. Dono: Davi.'
aliases:
- distance matrix cache
- cache de tempos de viagem
- TTL do cache
- Redis cache
tags:
- projeto/rota-certa
edges: []
source:
  note: notes/decisoes/2026-10-28-cache-matriz.md
  hash: a50b86fb
  session: 01M69V345Z9XT1ETEFPJADPQ25
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:41:38Z
updated_at: 2026-10-31T20:43:00Z
---

Decisão inicial (2026-10-28): cache no Redis com TTL de 6h, pois calcular a matriz completa leva ~4 min.

Revisão (2026-10-30, após INC-042): TTL reduzido para 1h e cache invalidado imediatamente ao chegar boletim de interdição. Custo de ~2 min por rodada, aceito pelo time. Davi baixa o TTL e adiciona a invalidação; Lívia assina o feed de boletins no `otimizador` até 07/11.

A versão de 6h é obsoleta.
