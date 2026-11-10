---
id: 01M69VHF8RG1Q65YZ2CWPQATMB
type: procedure
title: Deploy do otimizador (janela e canary 25%)
summary: 'Deploy só entre 10h e 16h e nunca na sexta (o plano roda às 3h30): CI verde, tag, canary com 25% dos pedidos de ontem (diferença de km >3% aborta), promover com rc-admin; o plantão acompanha a rodada das 3h30.'
aliases:
- deploy runbook
- publicar otimizador
- rollback otimizador
- release otimizador
tags:
- runbook
- procedimento
edges:
- to: 01M69V4WDRD1MY96Q8SW1HEJ44
  type: supersedes
- to: note:runbooks/deploy-otimizador.md
  type: derived_from
source:
  note: notes/runbooks/deploy-otimizador.md
  hash: 80ea1979
  session: 01M69VH0X6V6R1GNFVT7HM51YG
author: claude-code
confirmed_by: human
confirmed_at: 2026-10-31
valid_from: 2026-10-31
valid_until: null
origin: null
created_at: 2026-10-31T20:48:40Z
updated_at: 2026-10-31T20:49:21Z
---

Janela: apenas 10h–16h, nunca sexta, pois o plano roda às 3h30.

1. Merge em `main` com CI verde.
2. Tag da imagem `otimizador:YYYYMMDD-n`.
3. Canary: shadow plan com 25% dos pedidos de ontem, comparando km total com produção; diferença >3% interrompe o deploy.
4. Promover: `rc-admin deploy otimizador --tag <tag>`.
5. Acompanhar a rodada das 3h30 da manhã seguinte; o plantão é o dono.

Rollback: `rc-admin deploy otimizador --tag <anterior>`.

Mudança em relação à versão anterior: o canary subiu de 10% para 25% dos pedidos de ontem, conforme a nota atual.
