---
id: 01M69V4WDRD1MY96Q8SW1HEJ44
type: procedure
title: Deploy do otimizador (janela e canary)
summary: 'Deploy só entre 10h e 16h e nunca na sexta (o plano roda às 3h30): CI verde, tag, canary com 10% dos pedidos (diferença de km >3% aborta), promover com rc-admin; o plantão acompanha a rodada das 3h30.'
aliases:
- deploy runbook
- publicar otimizador
- rollback otimizador
- release otimizador
tags:
- runbook
- procedimento
edges: []
source:
  note: notes/runbooks/deploy-otimizador.md
  hash: 8ea38f3a
  session: 01M69V345Z9XT1ETEFPJADPQ25
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:41:47Z
updated_at: 2026-10-31T20:43:00Z
---

Janela: apenas 10h–16h, nunca sexta, pois o plano roda às 3h30.

1. Merge em `main` com CI verde.
2. Tag da imagem `otimizador:YYYYMMDD-n`.
3. Canary: shadow plan com 10% dos pedidos de ontem, comparando km total com produção; diferença >3% interrompe o deploy.
4. Promover: `rc-admin deploy otimizador --tag <tag>`.
5. Acompanhar a rodada das 3h30 da manhã seguinte; o plantão é o dono.

Rollback: `rc-admin deploy otimizador --tag <anterior>`.

Nota: o deploy do novo otimizador (jsprit) foi adiado para terça por causa da regra da sexta.
