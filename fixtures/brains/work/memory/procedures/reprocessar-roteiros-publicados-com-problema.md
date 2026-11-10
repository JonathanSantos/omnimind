---
id: 01M69V4SQ9SC9703GDSX0NMVY9
type: procedure
title: Reprocessar roteiros publicados com problema
summary: 'Para rota com problema após as 4h: pausar despacho, limpar cache da matriz, replanejar só veículos que não saíram, Ricardo aprova, retomar e registrar planos antigo/novo. Nunca replanejar sem limpar o cache.'
aliases:
- rerun plan
- replanejar rotas
- reprocess routes
- runbook reprocessamento
tags:
- runbook
- procedimento
edges: []
source:
  note: notes/runbooks/reprocessar-roteiros.md
  hash: 3e0cb7ea
  session: 01M69V345Z9XT1ETEFPJADPQ25
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:41:44Z
updated_at: 2026-10-31T20:43:00Z
---

Quando usar: rota publicada com problema depois das 4h (ex.: INC-042).

1. Avise o Ricardo e pause o despacho: `rc-admin despacho pause`.
2. Invalide o cache: `rc-admin matriz flush --regiao guarulhos`.
3. Gere novo plano só para veículos que ainda não saíram: `rc-admin plano replan --desde agora`.
4. Ricardo aprova o plano novo na tela de revisão.
5. Retome: `rc-admin despacho resume`.
6. Registre no canal de incidentes o número do plano antigo e do novo.

Não rode o passo 3 sem o 2: o plano sai igual ao anterior.
