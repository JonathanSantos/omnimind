---
id: 01M69VFFQEJDVJ0QJ9J97BYEA6
type: decision
title: 'Cache da matriz de distâncias: Redis com TTL curto (15 min provisório)'
summary: Matriz cacheada no Redis porque calcular a completa leva 4 min; TTL foi de 6h (28/10) para 1h após o INC-042 e para 15 min em 31/10, até o feed de boletins de interdição entrar no ar; invalidação por boletim está suspensa.
aliases:
- distance matrix cache
- cache de tempos de viagem
- TTL do cache da matriz
- matrix cache TTL
- invalidação de cache por boletim
tags:
- rota-certa
- cache
- decisao
edges:
- to: 01M69V4Q3ZNF5QRVG7813E3B8P
  type: derived_from
- to: 01M69V4SQ9SC9703GDSX0NMVY9
  type: relates_to
source:
  note: notes/decisoes/2026-10-28-cache-matriz.md
  hash: 6f5e40cf
  session: 01M69VF0QWAJHE69TT74RZ9JGP
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:47:35Z
updated_at: 2026-10-31T20:48:15Z
---

Histórico da decisão:
- 2026-10-28: cache no Redis, TTL 6h, porque a matriz completa leva ~4 min.
- 2026-10-30 (pós INC-042): TTL 1h, com invalidação imediata ao chegar boletim de interdição da central de trânsito. Custo extra ~2 min por rodada, aceito pelo time.
- 2026-10-31: o boletim ainda não chega automaticamente, então o TTL caiu para 15 min até o feed entrar no ar. A invalidação por boletim fica suspensa.

Estado atual: TTL 15 min, sem invalidação por boletim. Reverter para 1h + invalidação quando o feed existir (a nota não diz quem é o dono nem a data de revisão).
